use tracing::{debug, error, trace};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use std::time::Duration;
use std::{process, thread};
use url::Url;
use tokio::sync::mpsc;
use tokio::task;


#[tokio::main]
async fn main() -> Result<(), tracing_loki::Error> {
    let (layer, task) = tracing_loki::builder()
        .label("host", "mine")?
        .extra_field("pid", format!("{}", process::id()))?
        .build_url(Url::parse("http://127.0.0.1:3100").unwrap())?;

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


    let (tx_a, mut rx_a) = mpsc::channel(1);
    let (tx_b, mut rx_b) = mpsc::channel(1);

    // 初始時啟動 A
    tx_a.send(()).await.unwrap();
    
    let handle_a = task::spawn(async move  {
        for _ in 0..50 {
            rx_a.recv().await;
            tracing::info!("A");
            thread::sleep(Duration::from_secs(1));
            tx_b.send(()).await.unwrap(); 
        }
    });

    let handle_b = task::spawn(async move  {
        for _ in 0..50 {
            rx_b.recv().await;
            tracing::info!("B");
            thread::sleep(Duration::from_secs(1));
            tx_a.send(()).await.unwrap(); 
        }
    });


    handle_a.await.unwrap();
    handle_b.await.unwrap();


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