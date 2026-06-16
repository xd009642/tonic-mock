use codegen::primary_client::PrimaryClient;
use codegen::primary_mock::MockPrimary;
use codegen::secondary_client::SecondaryClient;
use codegen::secondary_mock::MockSecondary;
use codegen::stream_only_client::StreamOnlyClient;
use codegen::stream_only_mock::MockStreamOnly;
use codegen::{LocalReply, LocalRequest, WatchReply, WatchRequest};
use futures::StreamExt;
use shared::{SharedReply, SharedRequest};
use tonic::Request;
use tonic_mock::prelude::*;

pub mod shared {
    tonic::include_proto!("shared");
}

pub mod codegen {
    tonic::include_proto!("codegen");
}

/// Verifies that generated mocks compile and run when method request/response
/// types come from an imported protobuf package.
#[tokio::test]
async fn mocks_cross_package_unary_types() {
    let server = MockPrimary::builder()
        .mock_get(
            Mock::given(request_matches(|request: &SharedRequest| {
                request.value == "request"
            }))
            .expect(1)
            .respond_with(FixedResponse::ok(SharedReply {
                value: "reply".to_owned(),
            })),
        )
        .serve()
        .await;

    let mut client = PrimaryClient::connect(server.endpoint().await.unwrap())
        .await
        .unwrap();

    let response = client
        .get(Request::new(SharedRequest {
            value: "request".to_owned(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(response.value, "reply");
    server.verify().await.unwrap();
}

/// Verifies that a single protobuf package can generate independent mock
/// modules for multiple services without name collisions.
#[tokio::test]
async fn generates_independent_mocks_for_multiple_services() {
    let primary = MockPrimary::builder()
        .mock_get(
            Mock::given(request_matches(|_: &SharedRequest| true))
                .expect(1)
                .respond_with(FixedResponse::ok(SharedReply {
                    value: "primary".to_owned(),
                })),
        )
        .serve()
        .await;

    let secondary = MockSecondary::builder()
        .mock_ping(
            Mock::given(request_matches(|request: &LocalRequest| {
                request.value == "secondary"
            }))
            .expect(1)
            .respond_with(FixedResponse::ok(LocalReply {
                value: "pong".to_owned(),
            })),
        )
        .serve()
        .await;

    let mut primary_client = PrimaryClient::connect(primary.endpoint().await.unwrap())
        .await
        .unwrap();
    let mut secondary_client = SecondaryClient::connect(secondary.endpoint().await.unwrap())
        .await
        .unwrap();

    let primary_response = primary_client
        .get(Request::new(SharedRequest::default()))
        .await
        .unwrap()
        .into_inner();
    let secondary_response = secondary_client
        .ping(Request::new(LocalRequest {
            value: "secondary".to_owned(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(primary_response.value, "primary");
    assert_eq!(secondary_response.value, "pong");
    primary.verify().await.unwrap();
    secondary.verify().await.unwrap();
}

/// Verifies that a service with no unary methods still gets a usable mock for
/// server-streaming methods.
#[tokio::test]
async fn supports_server_streaming_on_streaming_only_services() {
    let server = MockStreamOnly::builder()
        .mock_subscribe(
            Mock::given(request_matches(|request: &WatchRequest| {
                request.topic == "builds"
            }))
            .expect(1)
            .respond_with_stream(StreamResponse::from_iter([
                WatchReply {
                    event: "queued".to_owned(),
                },
                WatchReply {
                    event: "done".to_owned(),
                },
            ])),
        )
        .serve()
        .await;

    let mut client = StreamOnlyClient::connect(server.endpoint().await.unwrap())
        .await
        .unwrap();

    let events = client
        .subscribe(Request::new(WatchRequest {
            topic: "builds".to_owned(),
        }))
        .await
        .unwrap()
        .into_inner()
        .map(|item| item.unwrap().event)
        .collect::<Vec<_>>()
        .await;

    assert_eq!(events, vec!["queued", "done"]);
    server.verify().await.unwrap();
}

/// Verifies that unsupported inbound streaming RPC shapes are still generated
/// as tonic service methods and fail with a clear unimplemented status.
#[tokio::test]
async fn unsupported_streaming_shapes_return_unimplemented() {
    let server = MockStreamOnly::builder().serve().await;
    let mut client = StreamOnlyClient::connect(server.endpoint().await.unwrap())
        .await
        .unwrap();

    let upload_error = client
        .upload(futures::stream::iter([LocalRequest {
            value: "one".to_owned(),
        }]))
        .await
        .unwrap_err();

    let chat_error = client
        .chat(futures::stream::iter([LocalRequest {
            value: "one".to_owned(),
        }]))
        .await
        .unwrap_err();

    assert_eq!(upload_error.code(), tonic::Code::Unimplemented);
    assert_eq!(chat_error.code(), tonic::Code::Unimplemented);
}
