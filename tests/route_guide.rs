use futures::StreamExt;
use routeguide::route_guide_client::RouteGuideClient;
use routeguide::route_guide_mock::MockRouteGuide;
use routeguide::{Feature, Point, Rectangle};
use tonic::{Request, Status};
use tonic_mock::prelude::*;

pub mod routeguide {
    tonic::include_proto!("routeguide");
}

fn point(latitude: i32, longitude: i32) -> Point {
    Point {
        latitude,
        longitude,
    }
}

fn feature(name: &str, latitude: i32, longitude: i32) -> Feature {
    Feature {
        name: name.to_owned(),
        location: Some(point(latitude, longitude)),
    }
}

#[tokio::test]
async fn unary_mock_verifies_matching_requests() {
    let server = MockRouteGuide::builder()
        .mock_get_feature(
            Mock::given(metadata_exists("grpc-trace"))
                .expect(1)
                .respond_with(FixedResponse::ok(feature("Mount Everest", 28, 87))),
        )
        .serve()
        .await;

    let mut client = RouteGuideClient::connect(server.endpoint().await.unwrap())
        .await
        .unwrap();

    let mut request = Request::new(point(2, 2));
    request
        .metadata_mut()
        .insert("grpc-trace", "trace me".try_into().unwrap());

    let response = client.get_feature(request).await.unwrap().into_inner();
    assert_eq!(response.name, "Mount Everest");
    server.verify().await.unwrap();
}

#[tokio::test]
async fn unary_mock_reports_mismatched_requests() {
    let server = MockRouteGuide::builder()
        .mock_get_feature(
            Mock::given(metadata_exists("grpc-trace"))
                .expect(1)
                .respond_with(FixedResponse::ok(feature("Mount Everest", 28, 87))),
        )
        .serve()
        .await;

    let mut client = RouteGuideClient::connect(server.endpoint().await.unwrap())
        .await
        .unwrap();

    client.get_feature(Request::new(point(2, 2))).await.unwrap();

    let error = server.verify().await.unwrap_err();
    assert!(error
        .to_string()
        .contains("get_feature expected calls == 1, got 0"));
    assert!(error
        .to_string()
        .contains("get_feature received at least one mismatched request"));
}

#[tokio::test]
async fn reset_clears_observed_unary_state() {
    let server = MockRouteGuide::builder()
        .mock_get_feature(
            Mock::given(metadata_exists("grpc-trace"))
                .expect(1)
                .respond_with(FixedResponse::ok(feature("Mount Everest", 28, 87))),
        )
        .serve()
        .await;

    let mut client = RouteGuideClient::connect(server.endpoint().await.unwrap())
        .await
        .unwrap();

    client.get_feature(Request::new(point(2, 2))).await.unwrap();
    assert!(server.verify().await.is_err());

    server.reset().await;

    let mut request = Request::new(point(2, 2));
    request
        .metadata_mut()
        .insert("grpc-trace", "trace me".try_into().unwrap());
    client.get_feature(request).await.unwrap();

    server.verify().await.unwrap();
}

#[tokio::test]
async fn unconfigured_methods_return_unimplemented() {
    let server = MockRouteGuide::builder().serve().await;
    let mut client = RouteGuideClient::connect(server.endpoint().await.unwrap())
        .await
        .unwrap();

    let error = client
        .get_feature(Request::new(point(2, 2)))
        .await
        .unwrap_err();
    assert_eq!(error.code(), tonic::Code::Unimplemented);
}

#[tokio::test]
async fn server_streaming_mock_returns_configured_stream() {
    let server = MockRouteGuide::builder()
        .mock_list_features(
            Mock::given(request_matches(|rectangle: &Rectangle| {
                rectangle.lo.as_ref().map(|p| p.latitude) == Some(1)
            }))
            .expect(1)
            .respond_with_stream(StreamResponse::from_iter([
                feature("first", 1, 2),
                feature("second", 3, 4),
            ])),
        )
        .serve()
        .await;

    let mut client = RouteGuideClient::connect(server.endpoint().await.unwrap())
        .await
        .unwrap();

    let response = client
        .list_features(Request::new(Rectangle {
            lo: Some(point(1, 1)),
            hi: Some(point(5, 5)),
        }))
        .await
        .unwrap();

    let names = response
        .into_inner()
        .map(|item| item.unwrap().name)
        .collect::<Vec<_>>()
        .await;

    assert_eq!(names, vec!["first", "second"]);
    server.verify().await.unwrap();
}

#[tokio::test]
async fn server_streaming_mock_can_return_status() {
    let server = MockRouteGuide::builder()
        .mock_list_features(
            Mock::given(request_matches(|_: &Rectangle| true))
                .expect(1)
                .respond_with_stream(StatusResponse::new(Status::unavailable("try later"))),
        )
        .serve()
        .await;

    let mut client = RouteGuideClient::connect(server.endpoint().await.unwrap())
        .await
        .unwrap();

    let error = client
        .list_features(Request::new(Rectangle::default()))
        .await
        .unwrap_err();

    assert_eq!(error.code(), tonic::Code::Unavailable);
    server.verify().await.unwrap();
}
