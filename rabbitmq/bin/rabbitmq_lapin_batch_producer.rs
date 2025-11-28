use lapin::{
    options::*,
    types::FieldTable,
    BasicProperties, Connection, ConnectionProperties,
};
use std::time::Duration;
use tokio::time::sleep;

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

    println!(" [*] Sending test messages for batch processing...");

    // Send messages in batches to test the batch consumer
    for batch_num in 1..=3 {
        println!("\n📦 Sending batch {} of messages:", batch_num);
        
        for msg_num in 1..=7 {
            let message = format!("Batch {} - Message {}", batch_num, msg_num);
            
            channel
                .basic_publish(
                    "",
                    queue_name,
                    BasicPublishOptions::default(),
                    message.as_bytes(),
                    BasicProperties::default(),
                )
                .await?;
                
            println!("  ✉️  Sent: {}", message);
            
            // Small delay between messages within a batch
            sleep(Duration::from_millis(50)).await;
        }
        
        // Longer delay between batches
        println!("  ⏸️  Waiting before next batch...");
        sleep(Duration::from_millis(2000)).await;
    }

    println!("\n✅ All test messages sent!");
    println!("💡 You can now run the batch consumer to see batch processing in action:");
    println!("   cargo run --bin rabbitmq_lapin_batch_reader");

    Ok(())
}