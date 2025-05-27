use lapin::{
    options::*,
    types::FieldTable,
    BasicProperties, Connection, ConnectionProperties,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to RabbitMQ
    let conn = Connection::connect(
        "amqp://127.0.0.1:5672",
        ConnectionProperties::default(),
    )
    .await?;

    let channel = conn.create_channel().await?;

    // 1. Create Dead-Letter Exchange and Queue
    let dlx_name = "dlx";
    let dlq_name = "dlq";

    // Declare Dead-Letter Exchange
    channel
        .exchange_declare(
            dlx_name,
            lapin::ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // Declare Dead-Letter Queue
    channel
        .queue_declare(
            dlq_name,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // Bind DLQ to DLX
    channel
        .queue_bind(
            dlq_name,
            dlx_name,
            "",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // 2. Create Main Queue with DLX Configuration
    let main_queue = "main_queue";
    let mut args = FieldTable::default();
    args.insert(
        "x-dead-letter-exchange".into(),
        lapin::types::AMQPValue::LongString(dlx_name.into()),
    );
    args.insert(
        "x-dead-letter-routing-key".into(),
        lapin::types::AMQPValue::LongString("".into()),
    );

    channel
        .queue_declare(
            main_queue,
            QueueDeclareOptions::default(),
            args,
        )
        .await?;

    // 3. Publish a Message
    channel
        .basic_publish(
            "",
            main_queue,
            BasicPublishOptions::default(),
            b"Hello World!",
            BasicProperties::default(),
        )
        .await?
        .await?;

    println!("Published message to main queue");

    // 4. Consume and Reject the Message
    let consumer = channel
        .basic_consume(
            main_queue,
            "my_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    println!("Waiting for messages... (Ctrl+C to exit)");

    consumer.set_delegate(move |delivery: Result<Option<lapin::message::Delivery>, lapin::Error>| {
        async move {
            let delivery = delivery.expect("error in consumer").unwrap();
            let message = String::from_utf8(delivery.data.clone()).unwrap();
            println!("Received message: {:?}", message );

            // Explicitly reject the message without requeue
            delivery
                .reject(BasicRejectOptions { requeue: false })
                .await
                .expect("Failed to reject message");

            println!("Message rejected and sent to DLQ");
        }
    });

    // 5. Consume from Dead-Letter Queue
    let dlq_consumer = channel
        .basic_consume(
            dlq_name,
            "dlq_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    println!("Waiting...");

    dlq_consumer.set_delegate(move |delivery: Result<Option<lapin::message::Delivery>, lapin::Error>| {
        async move {
            let delivery = delivery.expect("error in consumer").unwrap();
            let message = String::from_utf8(delivery.data.clone()).unwrap();
            println!("Received message in DLQ: {:?}", message );
            delivery.ack(BasicAckOptions::default()).await.expect("Failed to ack");
        }
    });
    println!("Waiting...");

    // Keep the program running
    tokio::signal::ctrl_c().await?;
    Ok(())
}