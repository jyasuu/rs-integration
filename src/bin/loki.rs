use tracing::{debug, error, trace};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use std::process;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), tracing_loki::Error> {
    let (layer, task) = tracing_loki::builder()
        .label("host", "mine")?
        .extra_field("pid", format!("{}", process::id()))?
        .build_url(Url::parse("http://localhost:3100").unwrap())?;

    // We need to register our layer with `tracing`.
    tracing_subscriber::registry()
        .with(LevelFilter::TRACE)
        .with(layer)
        // One could add more layers here, for example logging to stdout:
        .with(tracing_subscriber::fmt::Layer::new())
        .init();

    // The background task needs to be spawned so the logs actually get
    // delivered.
    tokio::spawn(task);

    tracing::info!(
        task = "tracing_setup",
        result = "success",
        "tracing successfully set up",
    );

    let pos = Position { x: 3.234, y: -1.223 };
    let origin_dist = pos.dist(Position::ORIGIN);

    debug!(?pos.x, ?pos.y);
    debug!(target: "app_events", position = ?pos, "New position");
    debug!(name: "completed", position = ?pos);

    trace!(position = ?pos, ?origin_dist);
    trace!(
        target: "app_events",
        position = ?pos,
        "x is {} and y is {}",
        if pos.x >= 0.0 { "positive" } else { "negative" },
        if pos.y >= 0.0 { "positive" } else { "negative" }
    );
    trace!(name: "completed", position = ?pos);

        
    let (err_info, port) = ("No connection", 22);

    error!(port, error = %err_info);
    error!(target: "app_events", "App Error: {}", err_info);
    error!({ info = err_info }, "error on port: {}", port);
    error!(name: "invalid_input", "Invalid input: {}", err_info);




    Ok(())
}


#[derive(Debug)]
struct  Position
{
    x: f64,
    y: f64
}

impl Position
{
    const ORIGIN: Self = Self {x: 0.0, y: 0.0};
    pub fn dist(&self,pos: Self) -> f64 {
        (pos.x - self.x) .abs() + (pos.y - self.y) .abs()
    }
}