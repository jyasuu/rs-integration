import grpc from 'k6/net/grpc';
import { check, sleep } from 'k6';

const client = new grpc.Client();
client.load(null, 'example.proto');

// This is a placeholder. Replace with your gRPC server address.
const GRPC_SERVER_ADDRESS = 'localhost:50051';

export const options = {
    stages: [
      { duration: '1m30s', target: 10 },
      { duration: '1m30s', target: 20 },
      { duration: '1m30s', target: 30 },
      { duration: '1m30s', target: 40 },
      { duration: '1m30s', target: 50 },
      { duration: '1m30s', target: 60 },
      { duration: '1m30s', target: 70 },
      { duration: '1m30s', target: 80 },
      { duration: '1m30s', target: 90 },
      { duration: '1m30s', target: 100 },
  
    ],
  };
  

export default () => {
  client.connect(GRPC_SERVER_ADDRESS, {
    plaintext: true // Set to false if your server uses TLS
  });

  // Constructing a sample ComplexTypes message
  const data = {
    repeated_str: ["hello", "world"],
    map_values: {
      "key1": 10,
      "key2": 20
    },
    status: "COMPLETED", // or use the enum number: 2
    nested: {
      nested_field: "this is nested"
    },
    // Example for the 'oneof' field, choosing 'text_content'
    text_content: "Sample text content for oneof"
    // Or, for binary_content:
    // binary_content: new Uint8Array([1, 2, 3, 4]).buffer // k6 expects ArrayBuffer for bytes
  };

  const response = client.invoke('example.ExampleService/ProcessData', data);

  check(response, {
    'status is OK': (r) => r && r.status === grpc.StatusOK,
  });

  console.log(JSON.stringify(response.message));
  client.close();
  sleep(1);
};