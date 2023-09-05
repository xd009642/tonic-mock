# Design

The purpose of tonic-mock is to try and replicate the testing functionality we 
get from wiremock for gRPC servers. In particular making it easy to
behaviourally test more complicated streaming based protocols. So in short
the aim of this project is to create mock servers to test code using gRPC
clients.

So to replicate wiremock we want some key functionality:

1. Matchers - checking requests meet certain conditions
2. Spying - set expectations on the number of invocations of a method
3. Responses - either fixed or tailored based on the request input

 For unary requests these are relatively simple and comparative to mocking
 normal REST APIs. For streaming requests there's an edge of complexity.
 However, there is a layer of complexity in handling gRPC itself...

 ## Code Generation

 gRPC relies on code generation to turn protobuf service and message
 definitions into code. This will have to be leveraged in order to
 make something actually usable.

 As it covers a lot of design space I'll be using the gRPC route guide
 example service to show how things will look. This is shown in a
 minified form below:

 ```protobuf
service RouteGuide {
  rpc GetFeature(Point) returns (Feature) {}
  rpc ListFeatures(Rectangle) returns (stream Feature) {}
  rpc RecordRoute(stream Point) returns (RouteSummary) {}
  rpc RouteChat(stream RouteNote) returns (stream RouteNote) {}
}

message Point {
  int32 latitude = 1;
  int32 longitude = 2;
}

message Rectangle {
  Point lo = 1;
  Point hi = 2;
}

message Feature {
  string name = 1;
  Point location = 2;
}

message RouteNote {
  Point location = 1;
  string message = 2;
}

message RouteSummary {
  int32 point_count = 1;
  int32 feature_count = 2;
  int32 distance = 3;
  int32 elapsed_time = 4;
}
```

This will generate a tonic trait for services to implement which looks like:

```rust
#[async_trait]
pub trait RouteGuide {
    async fn get_feature(&self, request: Request<Point>) -> Result<Response<Feature>, Status>;

    type ListFeaturesStream = Pin<Box<dyn Stream<Item = Result<Feature, Status>> + Send + 'static>>;

    async fn list_features(
        &self, 
        request: Request<Rectangle>
    ) -> Result<Response<Self::ListFeaturesStream>, Status>;

    async fn record_route(
        &self, 
        request: Request<tonic::Streaming<Point>>
    ) -> Result<Response<RouteSummary>, Status>;

    type RouteChatStream = Pin<Box<dyn Stream<Item = Result<RouteNote, Status>> + Send + 'static>>;

    async fn route_chat(
        &self, 
        request: Request<tonic::Streaming<RouteNote>>
    ) -> Result<Response<Self::RouteChatStream>, Status>;
}
```

It is an aim to autogenerate a trait and server implementation for this called
something like `MockRouteGuideService` which will handle setting up the mocks.
Otherwise users need to create a server containing mock method types, checkers etc
and wire everything up which adds a bunch of added complexity.

This would be a server module attribute so would currently be enabled by adding
`tonic-mock` as a build dependency and doing something like this in your `build.rs`:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .server_mod_attribute("routeguide", "#[tonic_mock::mock]")
        .compile(
            &["tests/protos/routeguide/route_guide.proto"],
            &["tests/protos"],
        )?;
    Ok(())
}
```

## Method Mocks

There are 4 types of methods we'll have to work out how to mock:

1. Unary - client sends one message, server responds with one message
2. Client streaming - client sends multiple messages, gets one response
3. Server streaming - client sends one message, gets multiple responses
4. Bidirectional streaming - client and server both send multiple messages

Unary most closely resembles the wiremock approach, so it's the simplest by far.
For the streaming ones we'll have to think a bit about what we want to test.
Server streaming feels simpler than client streaming just intuitively, because
we can reuse the unary matchers, spies and potentially the responses.

### Unary Mocks

Below is a code sketch for a simple unary mock just to check a specific method
is only called with a trace header. We may want this to ensure that our
distributed tracing system is being fully utilised!

```rust
#[tokio::test]
#[traced_test]
async fn check_mocked_route_guide() {
    let mut mock = MockRouteGuideService::build();

    mock.mock_get_feature()
        .add_matcher(MetadataExistsMatcher::new("grpc-trace".into()))
        .response(FixedResponse::ok(Feature {
            name: "Mount Everest".to_string(),
            location: Some(Point {
                latitude: 28,
                longitude: 87,
            }),
        }));

    let server = mock.build();
    server.serve().await;

    let addr = server.listening_address().await.unwrap();

    info!("Connecting to server: http://{}", addr);
    let mut client = RouteGuideClient::connect(addr).await.unwrap();

    let mut req = Request::new(Point {
        latitude: 2,
        longitude: 2,
    });
    req.metadata_mut()
        .insert("grpc-trace", "trace me".try_into().unwrap());

    info!("Get feature pass test");
    client.get_feature(req).await.unwrap();

    info!("Verifying result");
    assert!(server.verify().await);

    info!("Reset mock server state");
    server.reset().await;

    info!("Run the failing test");
    let req = Request::new(Point {
        latitude: 2,
        longitude: 2,
    });
    client.get_feature(req).await.unwrap();

    assert!(!server.verify().await);
}
```

### Client Streaming Mocks

Looking at the signature of the client streaming generated trait we have:

```rust
async fn record_route(
    &self, 
    request: Request<tonic::Streaming<Point>>
) -> Result<Response<RouteSummary>, Status>;
```

And looking at `tonic::Streaming` we have two methods:

1. `Streaming::message` which returns an `Option<T>` so you can get all the messages from the stream
2. `Streaming::trailers` consumes all the messages and gets trailing metadata.

So from here we can see something for our metadata matching.

1. We'll want to check trailing and starting metadata
2. Some checkers may be header only or tailer only. Some may apply to both

Do we want any matches that take in all the messages and all the metadata..?

Also for working our our response based on the input we'll want to grab all the
input. For checkers which need the stream context we'll have to consider some sort
of broadcast channel potentially?

Lets think about how  the code might look for our test:

```rust

```rust
#[tokio::test]
#[traced_test]
async fn check_mocked_route_guide() {
    let mut mock = MockRouteGuideService::build();

    mock.mock_record_route()
        .add_matcher(MetadataExistsMatcher::new("grpc-trace".into()))
        .response(FixedResponse::ok(Feature {
            name: "Mount Everest".to_string(),
            location: Some(Point {
                latitude: 28,
                longitude: 87,
            }),
        }));

```
