fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .server_mod_attribute("routeguide", "#[tonic_mock::mock]")
        .compile(
            &["tests/protos/routeguide/route_guide.proto"],
            &["tests/protos"],
        )?;
    Ok(())
}
