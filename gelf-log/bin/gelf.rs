extern crate gelf;

use gelf::{Logger, Message, UdpBackend};

fn main() {
    // Set up logging
    let backend = UdpBackend::new("127.0.0.1:12201")
        .expect("Failed to create UDP backend");
    let mut logger = Logger::new(Box::new(backend)).expect("Failed to determine hostname");
    logger.set_default_metadata(String::from("facility"), String::from("example-rust-app"));

    // Create a (complex) message
    let mut message = Message::new(String::from("Custom message!"));
    message
        .set_full_message(String::from("The full message text is more descriptive"))
        .set_metadata("foo", String::from("bar"))
        .unwrap()
        .set_metadata("baz", String::from("bat"))
        .unwrap();

    // Log it
    logger.log_message(message);
}