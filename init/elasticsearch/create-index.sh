#!/bin/bash
until curl -sX PUT "http://localhost:9200/documents" -H 'Content-Type: application/json' -d'
{
  "mappings": {
    "properties": {
      "title": { "type": "text", "analyzer": "english" },
      "content": { "type": "text", "analyzer": "english" },
      "created_at": { "type": "date" }
    }
  }
}'; do
  sleep 1
done