use futures::stream::{self, Stream};
use routeguide::route_guide_client::RouteGuideClient;
use routeguide::route_guide_server::{RouteGuide, RouteGuideServer};
use routeguide::{Feature, Point, Rectangle, RouteNote, RouteSummary};
use std::error::Error;
use std::pin::Pin;
use std::time::Duration;
use tokio::time;
use tonic::transport::Channel;
use tonic::Status;
use tonic::{Request, Response};
use tonic_mock::prelude::*;

pub mod routeguide {
    tonic::include_proto!("routeguide");
}

fn make_mock_server() {}

#[derive(Default)]
pub struct MockRouteGuideService {
    get_feature_mock: Option<UnaryMethodMock<Point, Feature>>,
}

impl MockRouteGuideService {
    fn mock_get_feature(&mut self) -> &mut UnaryMethodMock<Point, Feature> {
        self.get_feature_mock = Some(UnaryMethodMock::default());
        self.get_feature_mock.as_mut().unwrap()
    }
}

#[tonic::async_trait]
impl RouteGuide for MockRouteGuideService {
    async fn get_feature(&self, request: Request<Point>) -> Result<Response<Feature>, Status> {
        if let Some(s) = self.get_feature_mock.as_ref() {
            s.process_request(request)
        } else {
            Err(tonic::Status::unimplemented(
                "get_feature is not implemented",
            ))
        }
    }

    type ListFeaturesStream = Pin<Box<dyn Stream<Item = Result<Feature, Status>> + Send + 'static>>;

    async fn list_features(
        &self,
        request: Request<Rectangle>,
    ) -> Result<Response<Self::ListFeaturesStream>, Status> {
        todo!()
    }

    async fn record_route(
        &self,
        request: Request<tonic::Streaming<Point>>,
    ) -> Result<Response<RouteSummary>, Status> {
        todo!()
    }

    type RouteChatStream = Pin<Box<dyn Stream<Item = Result<RouteNote, Status>> + Send + 'static>>;

    async fn route_chat(
        &self,
        request: Request<tonic::Streaming<RouteNote>>,
    ) -> Result<Response<Self::RouteChatStream>, Status> {
        todo!()
    }
}

#[tokio::test]
async fn check_mocked_route_guide() {
    let mut mock = MockRouteGuideService::default();

    mock.mock_get_feature()
        .add_matcher(MetadataExistsMatcher::new("grpc-trace-bin".into()))
        .response(FixedResponse::ok(Feature {
            name: "Mount Everest".to_string(),
            location: Some(Point {
                latitude: 28,
                longitude: 87,
            }),
        }));
}
