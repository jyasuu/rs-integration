use tonic::{transport::Server, Request, Response, Status};
use example::example_service_server::{ExampleService, ExampleServiceServer};
use example::{BasicTypes, ComplexTypes};

mod example {
    tonic::include_proto!("../../../../../grpc/src/protos/example");
}

#[derive(Debug, Default)]
pub struct MyExampleService {}

#[tonic::async_trait]
impl ExampleService for MyExampleService {
    async fn process_data(
        &self,
        request: Request<ComplexTypes>,
    ) -> Result<Response<BasicTypes>, Status> {
        let req = request.into_inner();
        println!("REQUEST={:?}", req);

        // Handle complex types
        let status = match req.status() {
            example::Status::Completed => "DONE".to_string(),
            _ => "PENDING".to_string(),
        };

        // Process oneof field
        let content = match req.content {
            Some(example::complex_types::Content::TextContent(t)) => t,
            Some(example::complex_types::Content::BinaryContent(b)) => format!("{:?}", b),
            None => String::from("No content"),
        };

        Ok(Response::new(BasicTypes {
            i32_val: 42,
            string_val: format!("Status: {}, Content: {}", status, content),
            // ... other fields
            ..Default::default()
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse()?;
    let service = MyExampleService::default();

    Server::builder()
        .add_service(ExampleServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}