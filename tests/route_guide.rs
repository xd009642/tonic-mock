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

fn make_mock_server() {}

#[derive(Debug)]
pub struct MockRouteGuideService {
    get_feature_mock: Option<UnaryMethodMock<Point, Feature>>,
}

#[tonic::async_trait]
impl RouteGuide for RouteGuideService {
    async fn get_feature(&self, request: Request<Point>) -> Result<Response<Feature>, Status> {
        if let Some(s) = self.get_feature_mock.as_ref() {
            s.process_request(request)
        } else {
            Err(tonic::Status::unimplemented(
                format!("{} is not implemented", #(item.sig.ident)),
            ))
        }
    }

    type ListFeaturesStream = ReceiverStream<Result<Feature, Status>>;

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
