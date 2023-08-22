#[cfg(test)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["tests/protos/route_guide.proto"], &["tests/protos"])?;
    Ok(())
}

#[cfg(not(test))]
fn main() {}
