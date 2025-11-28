use futures::StreamExt;
use lapin::{
    options::*,
    types::FieldTable,
    Connection, ConnectionProperties,
};
use std::collections::VecDeque;
use std::time::Duration;
use tokio::time::{interval, timeout};

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

/// A message wrapper that includes delivery information
#[derive(Debug)]
pub struct BatchMessage {
    pub data: Vec<u8>,
    pub delivery_tag: u64,
    pub routing_key: String,
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

    /// Drain the buffer and return all messages
    pub fn drain_batch(&mut self) -> Vec<BatchMessage> {
        let messages: Vec<BatchMessage> = self.buffer
            .drain(..)
            .map(|delivery| BatchMessage {
                data: delivery.data.clone(),
                delivery_tag: delivery.delivery_tag,
                routing_key: delivery.routing_key.as_str().to_string(),
            })
            .collect();

        messages
    }

    /// Process a batch of messages and optionally ACK them
    pub async fn process_batch<F, Fut>(
        &mut self,
        channel: &lapin::Channel,
        mut processor: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(Vec<BatchMessage>) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
    {
        if self.is_empty() {
            return Ok(());
        }

        let messages = self.drain_batch();
        let last_delivery_tag = messages.last().map(|m| m.delivery_tag);

        // Process the batch
        match processor(messages).await {
            Ok(_) => {
                // If processing succeeded and auto_ack is enabled, ACK all messages
                if self.config.auto_ack {
                    if let Some(tag) = last_delivery_tag {
                        channel
                            .basic_ack(tag, BasicAckOptions { multiple: true })
                            .await?;
                        println!("✅ Batch ACK'd up to delivery tag: {}", tag);
                    }
                }
            }
            Err(e) => {
                println!("❌ Batch processing failed: {}", e);
                // Could implement retry logic or dead letter handling here
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
        F: FnMut(Vec<BatchMessage>) -> Fut + Clone,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
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

    // Configure batch processing
    let batch_config = BatchConfig {
        max_batch_size: 5,
        max_wait_time: Duration::from_millis(1000),
        auto_ack: true,
    };

    // Create batch consumer
    let mut batch_consumer = BatchConsumer::new(channel, batch_config);

    // Define message processor
    let message_processor = |messages: Vec<BatchMessage>| async move {
        println!("\n🔄 Processing batch of {} messages:", messages.len());
        
        for (i, message) in messages.iter().enumerate() {
            let content = String::from_utf8_lossy(&message.data);
            println!("  {}. [{}] {}", i + 1, message.routing_key, content);
        }
        
        // Simulate some processing work
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        println!("✅ Batch processed successfully!\n");
        Ok::<(), Box<dyn std::error::Error>>(())
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