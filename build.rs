

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src/protos")
        .compile(&["proto/example.proto"], &["proto/"])?;
    Ok(())
}