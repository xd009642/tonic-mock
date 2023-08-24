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
