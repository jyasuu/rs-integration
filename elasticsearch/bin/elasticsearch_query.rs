use elasticsearch::{Elasticsearch, Error, SearchParts,
    http::transport::Transport
};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Elasticsearch::default();
    let transport = Transport::single_node("http://localhost:9200")?;
    let client = Elasticsearch::new(transport);

    // make a search API call
    let search_response = client
        .search(SearchParts::None)
        .body(json!({
            "query": {
                "match_all": {}
            }
        }))
        .allow_no_indices(true)
        .send()
        .await?;

    // get the HTTP response status code
    let status_code = search_response.status_code();

    // read the response body. Consumes search_response
    let response_body = search_response.json::<Value>().await?;

    // read fields from the response body
    let took = response_body["took"].as_i64().unwrap();

    Ok(())
}