// src/consumer.rs
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct MessageData {
    producer_id: usize,
    message_id: usize,
    timestamp: i64,
    data: String,
}

pub async fn run_consumer(consumer_id: usize, broker: &str, group_id: &str, topic: &str) -> Result<()> {
    println!("Starting consumer {} in group {}", consumer_id, group_id);
    
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", broker)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()?;

    consumer.subscribe(&[topic])?;

    loop {
        match consumer.recv().await {
            Err(e) => eprintln!("Consumer {} error: {:?}", consumer_id, e),
            Ok(m) => {
                if let Some(payload) = m.payload() {
                    match std::str::from_utf8(payload) {
                        Ok(json) => {
                            match serde_json::from_str::<MessageData>(json) {
                                Ok(data) => {
                                    println!("Consumer {} received message from producer {} (msg {}): partition {} - {}",
                                        consumer_id, 
                                        data.producer_id, 
                                        data.message_id,
                                        m.partition(),
                                        data.data
                                    );
                                },
                                Err(e) => eprintln!("Failed to parse JSON: {:?}", e),
                            }
                        },
                        Err(e) => eprintln!("Invalid UTF-8: {:?}", e),
                    }
                }
                
                // 自動提交offset (因為設置了enable.auto.commit=true)
                consumer.commit_message(&m, CommitMode::Async)?;
            }
        }
    }
}