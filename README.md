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