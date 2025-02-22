use std::fmt::Write;
use std::time::Duration;
use kafka::producer::{Producer, Record, RequiredAcks};

fn main() {


    let mut producer =
        Producer::from_hosts(vec!("localhost:9092".to_owned()))
            .with_ack_timeout(Duration::from_secs(1))
            .with_required_acks(RequiredAcks::One)
            .create()
            .unwrap();
    
    let mut buf = String::with_capacity(2);
    for i in 0..10 {
      let _ = write!(&mut buf, "{}", i); // some computation of the message data to be sent
      producer.send(&Record::from_value("my-topic", buf.as_bytes())).unwrap();
      buf.clear();
    }
}