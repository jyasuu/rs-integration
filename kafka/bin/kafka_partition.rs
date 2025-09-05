// src/main.rs
mod producer;
mod consumer;

use anyhow::Result;
use tokio::task;
use std::time::Duration;

const BROKER: &str = "localhost:9092";
const TOPIC: &str = "test-topic";
const GROUP_ID: &str = "test-group";

#[tokio::main]
async fn main() -> Result<()> {
    // 啟動多個生產者
    let producer_handles = vec![
        task::spawn(producer::run_producer(1, BROKER, TOPIC, 20)),
        task::spawn(producer::run_producer(2, BROKER, TOPIC, 15)),
        task::spawn(producer::run_producer(3, BROKER, TOPIC, 25)),
    ];

    // 等待一段時間讓生產者開始發送消息
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 啟動多個消費者
    let consumer_handles = vec![
        task::spawn(consumer::run_consumer(1, BROKER, GROUP_ID, TOPIC)),
        task::spawn(consumer::run_consumer(2, BROKER, GROUP_ID, TOPIC)),
        task::spawn(consumer::run_consumer(3, BROKER, GROUP_ID, TOPIC)),
    ];

    // 等待所有生產者完成
    for handle in producer_handles {
        let _ = handle.await;
    }

    // 等待一段時間讓消費者處理完所有消息
    println!("Producers finished, waiting for consumers...");
    tokio::time::sleep(Duration::from_secs(10)).await;

    // 取消消費者任務
    for handle in consumer_handles {
        handle.abort();
    }

    Ok(())
}