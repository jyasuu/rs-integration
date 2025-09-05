// src/producer.rs
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use anyhow::Result;
use serde::Serialize;
use std::time::Duration;
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[derive(Serialize, Debug)]
struct Message {
    producer_id: usize,
    message_id: usize,
    timestamp: i64,
    data: String,
}

pub async fn run_producer(producer_id: usize, broker: &str, topic: &str, count: usize) -> Result<()> {
    println!("Starting producer {} to send {} messages", producer_id, count);
    
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", broker)
        .set("message.timeout.ms", "5000")
        .create()?;

    // 為每個生產者創建獨立的隨機數生成器
    let mut rng = StdRng::from_entropy();
    
    for i in 0..count {
        let message = Message {
            producer_id,
            message_id: i,
            timestamp: chrono::Utc::now().timestamp_millis(),
            data: rng.clone()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(10)
                .map(char::from)
                .collect(),
        };
        
        let payload = serde_json::to_string(&message)?;
        let key = format!("producer-{}", i);
        
        let delivery_status: std::result::Result<(i32, i64), (rdkafka::error::KafkaError, rdkafka::message::OwnedMessage)> = producer.send(
            FutureRecord::to(topic)
                .key(&key)
                .payload(&payload),
            Duration::from_secs(0),
        ).await;
        
        match delivery_status {
            Ok(_) => println!("Producer {} sent message {}", producer_id, i),
            Err((e, _)) => eprintln!("Producer {} failed to send message {}: {:?}", producer_id, i, e),
        }
        
        // 使用可發送的隨機數生成器
        let delay = rng.gen_range(100..500);
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }
    
    println!("Producer {} finished", producer_id);
    Ok(())
}