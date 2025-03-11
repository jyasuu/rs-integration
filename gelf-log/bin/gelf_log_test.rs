#[macro_use]
extern crate log;

extern crate gelf;

use gelf::{Logger, UdpBackend, Message, Level};
use log::LevelFilter;

pub fn main() {
    let backend = UdpBackend::new("127.0.0.1:12201")
        .expect("Failed to create UDP backend");

    // Init logging system
    let logger = Logger::new(Box::new(backend))
        .expect("Failed to determine hostname");
    logger.install(LevelFilter::Trace)
        .expect("Failed to install logger");

    info!("Descend into our program!");
    somewhere()
}

pub fn somewhere() {
    trace!("Trace something here!");
    over::the_rainbow();
}

mod over {
    pub fn the_rainbow() {
        error!("Oh well...");
    }
}