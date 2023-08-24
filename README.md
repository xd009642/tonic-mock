# tonic-mock

## Plan

```rust
// In your build script add a proc macro attribute to the server mod. 
// This will generate the default Mock server for you
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .server_mod_attribute(tonic_mock::Mock) 
        .compile(&["tests/protos/route_guide.proto"], &["tests/protos"])?;
    Ok(())
}
```

## Prior Art

* [grpcmock (Go)](https://github.com/nhatthm/grpcmock)
* [grpcmock (Java)](https://github.com/Fadelis/grpcmock)
* [Wiremock (Rust)](https://github.com/LukeMathWalker/wiremock-rs)
