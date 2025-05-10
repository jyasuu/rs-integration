use example::{example_service_client::ExampleServiceClient, BasicTypes, ComplexTypes, NestedMessage, Status};
use std::collections::HashMap;

mod example {
    tonic::include_proto!("../../../../../grpc/src/protos/example");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ExampleServiceClient::connect("http://127.0.0.1:50051").await?;

    let request = ComplexTypes {
        repeated_str: vec!["item1".into(), "item2".into()],
        map_values: HashMap::from([("key1".into(), 1), ("key2".into(), 2)]),
        status: Status::Completed.into(),
        nested: Some(NestedMessage {
            nested_field: "nested_value".into(),
        }),
        content: Some(example::complex_types::Content::TextContent(
            "Hello gRPC".into(),
        )),
    };

    let response = client.process_data(request).await?;
    println!("RESPONSE={:?}", response);

    Ok(())
}