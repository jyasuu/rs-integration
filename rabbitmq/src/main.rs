fn main() {
    println!("🐰 RabbitMQ Examples in Rust");
    println!("");
    println!("Available examples:");
    println!("");
    
    println!("📦 BATCH PROCESSING (NEW!):");
    println!("  cargo run --bin rabbitmq_lapin_batch_reader    # Batch consumer with configurable processing");
    println!("  cargo run --bin rabbitmq_lapin_batch_producer  # Test producer for batch processing");
    println!("");
    
    println!("🔧 BASIC MESSAGING:");
    println!("  cargo run --bin rabbitmq_lapin_send           # Send messages");
    println!("  cargo run --bin rabbitmq_lapin_receive        # Receive messages");
    println!("  cargo run --bin rabbitmq_lapin_new_task       # Send work tasks");
    println!("  cargo run --bin rabbitmq_lapin_worker         # Process work tasks");
    println!("");
    
    println!("📡 PUBLISH/SUBSCRIBE:");
    println!("  cargo run --bin rabbitmq_lapin_emit_log             # Fanout exchange publisher");
    println!("  cargo run --bin rabbitmq_lapin_receive_logs         # Fanout exchange subscriber");
    println!("  cargo run --bin rabbitmq_lapin_emit_log_direct      # Direct exchange publisher");
    println!("  cargo run --bin rabbitmq_lapin_receive_logs_direct  # Direct exchange subscriber");
    println!("  cargo run --bin rabbitmq_lapin_emit_log_topic       # Topic exchange publisher");
    println!("  cargo run --bin rabbitmq_lapin_receive_logs_topic   # Topic exchange subscriber");
    println!("");
    
    println!("🔄 ADVANCED PATTERNS:");
    println!("  cargo run --bin rabbitmq_lapin_rpc_client     # RPC client");
    println!("  cargo run --bin rabbitmq_lapin_rpc_server     # RPC server");
    println!("  cargo run --bin rabbitmq_lapin_dead_letter    # Dead letter queue handling");
    println!("");
    
    println!("🌊 STREAM PROCESSING:");
    println!("  cargo run --bin rabbitmq_send_offset_tracking    # Stream publisher with offset tracking");
    println!("  cargo run --bin rabbitmq_receive_offset_tracking # Stream consumer with offset tracking");
    println!("");
    
    println!("💡 To get started with batch processing:");
    println!("  1. Start RabbitMQ: docker run -d --name rabbitmq -p 5672:5672 rabbitmq:3");
    println!("  2. Run batch consumer: cargo run --bin rabbitmq_lapin_batch_reader");
    println!("  3. In another terminal, run producer: cargo run --bin rabbitmq_lapin_batch_producer");
    println!("");
    println!("📖 See README.md for detailed documentation and configuration options.");
}
