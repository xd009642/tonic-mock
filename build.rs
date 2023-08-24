fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .server_mod_attribute("attrs", "#[tonic_mock::mock]")
        .build_client(true)
        .client_mod_attribute("attrs", "#[cfg(feature = \"client\")]")
        .out_dir(".")
        .compile(
            &["tests/protos/routeguide/route_guide.proto"],
            &["tests/protos"],
        )?;
    Ok(())
}
