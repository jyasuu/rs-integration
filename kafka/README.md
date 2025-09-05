
```bash
docker rm -f broker && docker compose up -d && docker exec -it broker /opt/kafka/bin/kafka-topics.sh --create --topic test-topic \
  --bootstrap-server localhost:9092 \
  --partitions 3 


```