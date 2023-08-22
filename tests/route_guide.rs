use futures::stream;
use rand::rngs::ThreadRng;
use rand::Rng;
use routeguide::route_guide_client::RouteGuideClient;
use routeguide::route_guide_server::{RouteGuide, RouteGuideServer};
use routeguide::{Point, Rectangle, RouteNote};
use std::error::Error;
use std::time::Duration;
use tokio::time;
use tonic::transport::Channel;
use tonic::Request;
use tonic_mock::prelude::*;

pub mod routeguide {
    tonic::include_proto!("routeguide");
}

#[derive(Debug)]
pub struct RouteGuideService {
    get_feature_mock: MethodMock<Point, Feature>,
}

#[tonic::async_trait]
impl RouteGuide for RouteGuideService {
    type ListFeaturesStream = ReceiverStream<Result<Feature, Status>>;
    type RouteChatStream = Pin<Box<dyn Stream<Item = Result<RouteNote, Status>> + Send + 'static>>;

    async fn get_feature(&self, request: Request<Point>) -> Result<Response<Feature>, Status> {
        self.get_feature_mock.process_request(request)
    }

    async fn list_features(
        &self,
        request: Request<Rectangle>,
    ) -> Result<Response<Self::ListFeaturesStream>, Status> {
    }

    async fn record_route(
        &self,
        request: Request<tonic::Streaming<Point>>,
    ) -> Result<Response<RouteSummary>, Status> {
    }

    async fn route_chat(
        &self,
        request: Request<tonic::Streaming<RouteNote>>,
    ) -> Result<Response<Self::RouteChatStream>, Status> {
    }
}

fn make_mock_server() {}
