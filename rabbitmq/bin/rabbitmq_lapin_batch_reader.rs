use futures::StreamExt;
use lapin::{
    options::*,
    types::FieldTable,
    Connection, ConnectionProperties,
};
use std::collections::VecDeque;
use std::time::Duration;
use tokio::time::interval;

/// Batch configuration for RabbitMQ consumer
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub max_wait_time: Duration,
    pub auto_ack: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 10,
            max_wait_time: Duration::from_millis(100),
            auto_ack: false,
        }
    }
}

/// A message wrapper that includes delivery information and original delivery for ACKing
#[derive(Debug)]
pub struct BatchMessage {
    pub data: Vec<u8>,
    pub delivery_tag: u64,
    pub routing_key: String,
    pub delivery: lapin::message::Delivery,
}

/// Results from processing a batch of messages
#[derive(Debug)]
pub enum BatchProcessResult {
    Success,
    PartialFailure(Vec<u64>), // delivery_tags of failed messages
    TotalFailure,
}

/// Batch processor for RabbitMQ messages
pub struct BatchProcessor {
    config: BatchConfig,
    buffer: VecDeque<lapin::message::Delivery>,
}

impl BatchProcessor {
    pub fn new(config: BatchConfig) -> Self {
        Self {
            config,
            buffer: VecDeque::new(),
        }
    }

    /// Add a message to the batch buffer
    pub fn add_message(&mut self, delivery: lapin::message::Delivery) {
        self.buffer.push_back(delivery);
    }

    /// Check if batch is ready for processing
    pub fn is_batch_ready(&self) -> bool {
        self.buffer.len() >= self.config.max_batch_size
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Drain the buffer and return all messages with their original delivery objects
    pub fn drain_batch(&mut self) -> Vec<BatchMessage> {
        let messages: Vec<BatchMessage> = self.buffer
            .drain(..)
            .map(|delivery| BatchMessage {
                data: delivery.data.clone(),
                delivery_tag: delivery.delivery_tag,
                routing_key: delivery.routing_key.as_str().to_string(),
                delivery,
            })
            .collect();

        messages
    }

    /// Process a batch of messages with proper ACK/NACK handling
    pub async fn process_batch<F, Fut>(
        &mut self,
        _channel: &lapin::Channel,
        mut processor: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&[BatchMessage]) -> Fut,
        Fut: std::future::Future<Output = Result<BatchProcessResult, Box<dyn std::error::Error>>>,
    {
        if self.is_empty() {
            return Ok(());
        }

        let messages = self.drain_batch();
        let message_count = messages.len();

        // Process the batch
        match processor(&messages).await {
            Ok(BatchProcessResult::Success) => {
                // ACK all messages individually or as a batch
                if self.config.auto_ack {
                    // Use batch ACK for efficiency when auto_ack is true
                    if let Some(last_message) = messages.last() {
                        last_message.delivery
                            .ack(BasicAckOptions { multiple: true })
                            .await?;
                        println!("✅ Batch ACK'd {} messages up to delivery tag: {}", 
                                message_count, last_message.delivery_tag);
                    }
                } else {
                    // ACK each message individually when auto_ack is false for better control
                    for message in &messages {
                        message.delivery
                            .ack(BasicAckOptions { multiple: false })
                            .await?;
                    }
                    println!("✅ Individually ACK'd {} messages", message_count);
                }
            }
            Ok(BatchProcessResult::PartialFailure(failed_tags)) => {
                // ACK successful messages, NACK failed ones
                for message in &messages {
                    if failed_tags.contains(&message.delivery_tag) {
                        // NACK and requeue failed messages
                        message.delivery
                            .nack(BasicNackOptions { 
                                multiple: false, 
                                requeue: true 
                            })
                            .await?;
                        println!("❌ NACK'd message with delivery tag: {}", message.delivery_tag);
                    } else {
                        // ACK successful messages
                        message.delivery
                            .ack(BasicAckOptions { multiple: false })
                            .await?;
                    }
                }
                println!("✅ Partial batch processed: {} succeeded, {} failed", 
                        message_count - failed_tags.len(), failed_tags.len());
            }
            Ok(BatchProcessResult::TotalFailure) => {
                // NACK all messages and requeue them
                for message in &messages {
                    message.delivery
                        .nack(BasicNackOptions { 
                            multiple: false, 
                            requeue: true 
                        })
                        .await?;
                }
                println!("❌ Total batch failure: NACK'd and requeued {} messages", message_count);
            }
            Err(e) => {
                println!("❌ Batch processing error: {}", e);
                // NACK all messages on processing error
                for message in &messages {
                    if let Err(nack_err) = message.delivery
                        .nack(BasicNackOptions { 
                            multiple: false, 
                            requeue: true 
                        })
                        .await {
                        println!("⚠️ Failed to NACK message {}: {}", message.delivery_tag, nack_err);
                    }
                }
                return Err(e);
            }
        }

        Ok(())
    }
}

/// Main batch consumer that handles message collection and batch processing
pub struct BatchConsumer {
    processor: BatchProcessor,
    channel: lapin::Channel,
}

impl BatchConsumer {
    pub fn new(channel: lapin::Channel, config: BatchConfig) -> Self {
        Self {
            processor: BatchProcessor::new(config),
            channel,
        }
    }

    /// Start consuming messages in batches
    pub async fn start_consuming<F, Fut>(
        &mut self,
        queue_name: &str,
        consumer_tag: &str,
        message_processor: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&[BatchMessage]) -> Fut + Clone,
        Fut: std::future::Future<Output = Result<BatchProcessResult, Box<dyn std::error::Error>>>,
    {
        // Create consumer
        let mut consumer = self
            .channel
            .basic_consume(
                queue_name,
                consumer_tag,
                BasicConsumeOptions {
                    no_ack: false, // We want manual ACK control
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        // Set up timer for batch timeout
        let mut batch_timer = interval(self.processor.config.max_wait_time);
        let mut message_processor = message_processor;

        println!(" [*] Waiting for messages in batches. To exit press CTRL+C");

        loop {
            tokio::select! {
                // Handle incoming messages
                delivery_result = consumer.next() => {
                    match delivery_result {
                        Some(Ok(delivery)) => {
                            println!(" [x] Received message, adding to batch (size: {})", 
                                   self.processor.buffer.len() + 1);
                            
                            self.processor.add_message(delivery);
                            
                            // Process batch if it's ready
                            if self.processor.is_batch_ready() {
                                println!(" [→] Batch size limit reached, processing batch...");
                                self.processor.process_batch(&self.channel, &mut message_processor).await?;
                            }
                        }
                        Some(Err(e)) => {
                            println!("❌ Error receiving message: {}", e);
                            continue;
                        }
                        None => {
                            println!(" [!] Consumer stream ended");
                            break;
                        }
                    }
                }
                
                // Handle batch timeout
                _ = batch_timer.tick() => {
                    if !self.processor.is_empty() {
                        println!(" [⏰] Batch timeout reached, processing partial batch...");
                        self.processor.process_batch(&self.channel, &mut message_processor).await?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::var("RABBITMQ_URL").unwrap_or_else(|_| "amqp://127.0.0.1:5672".to_string());
    
    // Connect to RabbitMQ
    let conn = Connection::connect(&addr, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;

    // Declare queue
    let queue_name = "batch_test_queue";
    channel
        .queue_declare(
            queue_name,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // Configure batch processing - demonstrate manual ACK control
    let auto_ack = std::env::var("AUTO_ACK").unwrap_or_else(|_| "false".to_string()).parse().unwrap_or(false);
    
    let batch_config = BatchConfig {
        max_batch_size: 5,
        max_wait_time: Duration::from_millis(1000),
        auto_ack,
    };

    println!("🔧 Batch configuration: max_size={}, timeout={}ms, auto_ack={}", 
             batch_config.max_batch_size, 
             batch_config.max_wait_time.as_millis(), 
             batch_config.auto_ack);

    // Create batch consumer
    let mut batch_consumer = BatchConsumer::new(channel, batch_config);

    // Define message processor with proper result handling
    let message_processor = |messages: &[BatchMessage]| {
        let message_count = messages.len();
        let mut failed_tags = Vec::new();
        
        // Extract data we need to avoid lifetime issues
        let message_data: Vec<(String, u64, String)> = messages.iter()
            .map(|m| (
                String::from_utf8_lossy(&m.data).to_string(),
                m.delivery_tag,
                m.routing_key.clone()
            ))
            .collect();
        
        async move {
            println!("\n🔄 Processing batch of {} messages:", message_count);
            
            for (i, (content, delivery_tag, routing_key)) in message_data.iter().enumerate() {
                println!("  {}. [{}] {}", i + 1, routing_key, content);
                
                // Simulate processing that might fail for some messages
                // For demo: fail messages containing "error"
                if content.to_lowercase().contains("error") {
                    println!("    ❌ Processing failed for message {}", delivery_tag);
                    failed_tags.push(*delivery_tag);
                }
            }
            
            // Simulate some processing work
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            let result = if failed_tags.is_empty() {
                println!("✅ Batch processed successfully!\n");
                BatchProcessResult::Success
            } else if failed_tags.len() == message_count {
                println!("❌ Total batch failure!\n");
                BatchProcessResult::TotalFailure
            } else {
                println!("⚠️ Partial batch failure: {} failed out of {}\n", failed_tags.len(), message_count);
                BatchProcessResult::PartialFailure(failed_tags)
            };
            
            Ok::<BatchProcessResult, Box<dyn std::error::Error>>(result)
        }
    };

    // Start consuming
    batch_consumer
        .start_consuming(queue_name, "batch_consumer", message_processor)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_batch_size, 10);
        assert_eq!(config.max_wait_time, Duration::from_millis(100));
        assert!(!config.auto_ack);
    }

    #[test]
    fn test_batch_processor_creation() {
        let config = BatchConfig::default();
        let processor = BatchProcessor::new(config);
        assert!(processor.is_empty());
        assert!(!processor.is_batch_ready());
    }

    #[tokio::test]
    async fn test_batch_processing_logic() {
        let config = BatchConfig {
            max_batch_size: 2,
            max_wait_time: Duration::from_millis(100),
            auto_ack: false,
        };
        
        let mut processor = BatchProcessor::new(config);
        assert!(processor.is_empty());
        
        // Test that batch becomes ready when max size is reached
        // Note: This test would require mock delivery objects in a full test
    }
}
