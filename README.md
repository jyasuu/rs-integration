# rs-integration


## kafka

```sh
docker compose exec broker /opt/kafka/bin/kafka-topics.sh --bootstrap-server localhost:9092 --create --topic my-topic
docker compose exec broker /opt/kafka/bin/kafka-console-producer.sh --bootstrap-server localhost:9092 --topic my-topic
docker compose exec broker /opt/kafka/bin/kafka-console-consumer.sh --bootstrap-server localhost:9092 --topic my-topic

cargo run --bin kafka_producer
cargo run --bin kafka_consumer
```


## hurl

```sh
cargo install hurl
hurl --test hurl/test.hurl
```

## clap

```sh
cargo run --bin clap -- 123 -ddd test -l
```

## rabbitmq

```sh
docker run -it --rm --name rabbitmq -p 5552:5552 -p 15672:15672 -p 5672:5672  \
    -e RABBITMQ_SERVER_ADDITIONAL_ERL_ARGS='-rabbitmq_stream advertised_host localhost' \
    rabbitmq:3.13    

docker exec rabbitmq rabbitmq-plugins enable rabbitmq_stream rabbitmq_stream_management 

cargo run --bin rabbitmq_lapin_receive
cargo run --bin rabbitmq_lapin_send

cargo run --bin rabbitmq_lapin_worker
cargo run --bin rabbitmq_lapin_new_task "hi" # specify a custom message

cargo run --bin rabbitmq_lapin_receive_logs
cargo run --bin rabbitmq_lapin_emit_log "hi" # specify a custom message

cargo run --bin rabbitmq_lapin_receive_logs_direct info error # specify log levels
cargo run --bin rabbitmq_lapin_emit_log_direct error "help!" # specify severity and custom message

cargo run --bin rabbitmq_lapin_receive_logs_topic kern.* # specify topic filter
cargo run --bin rabbitmq_lapin_emit_log_topic kern.mem "No memory left!" # specify topic and message

cargo run --bin rabbitmq_lapin_rpc_server
cargo run --bin rabbitmq_lapin_rpc_client
```


## elasticsearch

```rs
wget https://archive.org/download/stackexchange/stackoverflow.com-Posts.7z
wget https://archive.org/download/stackexchange-snapshot-2018-03-14/stackoverflow.com-Posts.7z
./target/debug/index_questions_answers --path Posts.xml

```


## mcp

https://github.com/modelcontextprotocol/inspector?tab=readme-ov-file#configuration

```sh
MCP_PROXY_FULL_ADDRESS=https://6277-jyasuu-rsintegration-g6bec4xgw4j.ws-us118.gitpod.io
npx -y @modelcontextprotocol/inspector 
https://8000-jyasuu-rsintegration-g6bec4xgw4j.ws-us118.gitpod.io
```