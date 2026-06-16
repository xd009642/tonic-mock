fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_mock_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &["tests/protos/routeguide/route_guide.proto"],
            &["tests/protos"],
        )?;
    Ok(())
}
