# tonic-mock

`tonic-mock` provides typed mock gRPC servers for black-box testing Rust
applications that use `tonic` clients.

The current implementation supports unary and server-streaming RPCs. Client
streaming and bidirectional streaming methods are generated as valid tonic
service methods, but they return `Status::unimplemented`.

## Build script

Use `tonic-mock-build` from `build.rs` to generate the regular tonic client and
server modules plus typed mock service modules.

```rust
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
```

## Example

```rust
use routeguide::route_guide_client::RouteGuideClient;
use routeguide::route_guide_mock::MockRouteGuide;
use routeguide::{Feature, Point};
use tonic::Request;
use tonic_mock::prelude::*;

pub mod routeguide {
    tonic::include_proto!("routeguide");
}

#[tokio::test]
async fn mocked_route_guide() {
    let server = MockRouteGuide::builder()
        .mock_get_feature(
            Mock::given(metadata_exists("grpc-trace"))
                .expect(1)
                .respond_with(FixedResponse::ok(Feature {
                    name: "Mount Everest".to_owned(),
                    location: Some(Point {
                        latitude: 28,
                        longitude: 87,
                    }),
                })),
        )
        .serve()
        .await;

    let mut client = RouteGuideClient::connect(server.endpoint().await.unwrap())
        .await
        .unwrap();

    let mut request = Request::new(Point {
        latitude: 2,
        longitude: 2,
    });
    request
        .metadata_mut()
        .insert("grpc-trace", "trace me".try_into().unwrap());

    client.get_feature(request).await.unwrap();
    server.verify().await.unwrap();
}
```

## Prior Art

* [wiremocket](https://github.com/xd009642/wiremocket)
* [grpcmock (Go)](https://github.com/nhatthm/grpcmock)
* [grpcmock (Java)](https://github.com/Fadelis/grpcmock)
* [Wiremock (Rust)](https://github.com/LukeMathWalker/wiremock-rs)
