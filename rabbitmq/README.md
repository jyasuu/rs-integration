# RabbitMQ Examples in Rust

This directory contains various RabbitMQ examples using the `lapin` library for AMQP 0.9.1 protocol and `rabbitmq-stream-client` for RabbitMQ Streams.

## Batch Processing Implementation

### New Features Added

#### 1. Batch Reader (`rabbitmq_lapin_batch_reader.rs`)

A comprehensive batch processing consumer that collects messages and processes them in configurable batches. Features include:

- **Configurable batch sizes**: Set maximum number of messages per batch
- **Timeout-based processing**: Process partial batches when timeout is reached
- **Manual ACK control**: Support for multiple ACK to acknowledge entire batches
- **Error handling**: Proper error handling with optional retry logic
- **Async processing**: Fully async implementation using tokio

**Key Components:**

- `BatchConfig`: Configuration for batch size, timeout, and ACK behavior
- `BatchMessage`: Wrapper for message data with delivery information
- `BatchProcessor`: Core logic for batch collection and processing
- `BatchConsumer`: Main consumer that orchestrates batch processing

**Usage:**
```bash
# Start the batch consumer
cargo run --bin rabbitmq_lapin_batch_reader

# In another terminal, send test messages
cargo run --bin rabbitmq_lapin_batch_producer
```

#### 2. Batch Producer (`rabbitmq_lapin_batch_producer.rs`)

A test producer that sends messages in patterns suitable for testing batch processing:

- Sends messages in controlled bursts
- Configurable delays between messages and batches
- Helps demonstrate batch timeout and size-based processing

### Configuration Options

```rust
BatchConfig {
    max_batch_size: 5,        // Process when 5 messages accumulated
    max_wait_time: Duration::from_millis(1000), // Or after 1 second timeout
    auto_ack: true,           // Automatically ACK successful batches
}
```

### Batch Processing Benefits

1. **Improved Throughput**: Process multiple messages together
2. **Reduced ACK Overhead**: Use multiple ACK to acknowledge entire batches
3. **Better Resource Utilization**: Batch database operations, API calls, etc.
4. **Flexible Processing**: Handle both time and size-based batching

### Example Output

```
 [*] Waiting for messages in batches. To exit press CTRL+C
 [x] Received message, adding to batch (size: 1)
 [x] Received message, adding to batch (size: 2)
 [x] Received message, adding to batch (size: 3)
 [x] Received message, adding to batch (size: 4)
 [x] Received message, adding to batch (size: 5)
 [→] Batch size limit reached, processing batch...

🔄 Processing batch of 5 messages:
  1. [] Batch 1 - Message 1
  2. [] Batch 1 - Message 2
  3. [] Batch 1 - Message 3
  4. [] Batch 1 - Message 4
  5. [] Batch 1 - Message 5
✅ Batch processed successfully!

✅ Batch ACK'd up to delivery tag: 5
```

## Other Examples

### Basic Messaging
- `rabbitmq_lapin_send.rs` / `rabbitmq_lapin_receive.rs`: Basic send/receive
- `rabbitmq_lapin_new_task.rs` / `rabbitmq_lapin_worker.rs`: Work queue pattern

### Publish/Subscribe
- `rabbitmq_lapin_emit_log.rs` / `rabbitmq_lapin_receive_logs.rs`: Fanout exchange
- `rabbitmq_lapin_emit_log_direct.rs` / `rabbitmq_lapin_receive_logs_direct.rs`: Direct exchange
- `rabbitmq_lapin_emit_log_topic.rs` / `rabbitmq_lapin_receive_logs_topic.rs`: Topic exchange

### Advanced Patterns
- `rabbitmq_lapin_rpc_client.rs` / `rabbitmq_lapin_rpc_server.rs`: RPC pattern
- `rabbitmq_lapin_dead_letter.rs`: Dead letter queue handling

### Stream Processing
- `rabbitmq_send_offset_tracking.rs` / `rabbitmq_receive_offset_tracking.rs`: RabbitMQ Streams with offset tracking

## Prerequisites

1. **RabbitMQ Server**: Install and run RabbitMQ locally or use Docker:
   ```bash
   docker run -d --name rabbitmq -p 5672:5672 -p 15672:15672 rabbitmq:3-management
   ```

2. **Environment Variables** (optional):
   ```bash
   export RABBITMQ_URL="amqp://127.0.0.1:5672"
   ```

## Running Examples

```bash
# Build all examples
cargo build

# Run specific example
cargo run --bin rabbitmq_lapin_batch_reader

# List all available examples
cargo run --bin | grep rabbitmq
```

## Architecture

The batch processing implementation follows these patterns:

1. **Separation of Concerns**: Clear separation between message collection, processing, and acknowledgment
2. **Configurable Behavior**: All timing and size parameters are configurable
3. **Error Handling**: Proper error propagation with options for retry logic
4. **Resource Management**: Efficient memory usage with VecDeque for message buffering
5. **Async/Await**: Full async support for high-performance message processing

## References

- [RabbitMQ Tutorials](https://www.rabbitmq.com/getstarted.html)
- [Lapin Documentation](https://docs.rs/lapin/)
- [RabbitMQ Consumer Acknowledgements](https://www.rabbitmq.com/docs/confirms#consumer-acks-multiple-parameter)
- [AMQP 0.9.1 Protocol](https://www.rabbitmq.com/amqp-0-9-1-reference.html)