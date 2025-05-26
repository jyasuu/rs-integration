use lapin::{options::*, types::FieldTable, BasicProperties, Connection, ConnectionProperties};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "amqp://127.0.0.1:5672";
    let conn = Connection::connect(addr, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;

    let mut arguments =  FieldTable::default();
    arguments.insert(String::from("x-max-priority").into(), lapin::types::AMQPValue::ShortShortUInt(5));

    channel
        .queue_declare(
            "hello",
            QueueDeclareOptions::default(),
            arguments,
        )
        .await?;

    let priorities = [4,0,2,1,3];

    for priority in priorities
    {
        let properties =  BasicProperties::default();
        let properties = properties.with_priority(priority);

        let message = format!("Hello world {}!",priority);
        let payload = message.as_bytes();
        channel
            .basic_publish(
                "",
                "hello",
                BasicPublishOptions::default(),
                payload,
                properties,
            )
            .await?;

        println!(" [x] Sent \"{}\"",message);


    }

    
    conn.close(0, "").await?;

    Ok(())
}