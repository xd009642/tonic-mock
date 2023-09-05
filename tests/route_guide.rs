use futures::stream::{self, Stream};
use routeguide::route_guide_client::RouteGuideClient;
use routeguide::route_guide_server::{RouteGuide, RouteGuideServer};
use routeguide::{Feature, Point, Rectangle, RouteNote, RouteSummary};
use std::error::Error;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time;
use tonic::transport::server::{Server, TcpIncoming};
use tonic::transport::Channel;
use tonic::Status;
use tonic::{Request, Response};
use tonic_mock::prelude::*;
use tracing::info;
use tracing_test::traced_test;

pub mod routeguide {
    tonic::include_proto!("routeguide");
}

fn make_mock_server() {}

#[derive(Default)]
pub struct MockRouteGuideServiceBuilder {
    get_feature_mock: Option<UnaryMethodMock<Point, Feature>>,
}

#[derive(Default, Clone)]
pub struct MockRouteGuideService {
    get_feature_mock: Arc<RwLock<Option<UnaryMethodMock<Point, Feature>>>>,
    server_handle: Arc<RwLock<Option<Handle>>>,
}

pub struct Handle {
    addr: SocketAddr,
    tx: mpsc::Sender<()>,
}

impl Drop for Handle {
    fn drop(&mut self) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            tx.send(()).await.unwrap();
        });
    }
}

impl MockRouteGuideServiceBuilder {
    fn mock_get_feature(&mut self) -> &mut UnaryMethodMock<Point, Feature> {
        self.get_feature_mock = Some(UnaryMethodMock::default());
        self.get_feature_mock.as_mut().unwrap()
    }

    fn build(self) -> MockRouteGuideService {
        MockRouteGuideService {
            get_feature_mock: Arc::new(RwLock::new(self.get_feature_mock)),
            server_handle: Arc::default(),
        }
    }
}

impl MockRouteGuideService {
    pub fn build() -> MockRouteGuideServiceBuilder {
        Default::default()
    }

    pub async fn serve(&self) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind an OS port for a mock server.");
        let addr = listener.local_addr().unwrap();
        info!("Bound to: {:?}", addr);

        let to_serve = self.clone();

        let (tx, mut rx) = mpsc::channel(1);

        let server = tokio::spawn(async move {
            info!("Creating listener");
            let listener = TcpIncoming::from_listener(listener, true, None).unwrap();
            info!("Creating server");
            Server::builder()
                .add_service(RouteGuideServer::new(to_serve))
                .serve_with_incoming_shutdown(listener, async move {
                    let _ = rx.recv().await;
                })
                .await
                .unwrap();

            info!("Server closing down");
        });

        time::sleep(time::Duration::from_secs(1)).await;

        info!("Returning handle");
        self.server_handle.write().await.insert(Handle { tx, addr });
    }

    pub async fn verify(&self) -> bool {
        let read = self.get_feature_mock.read().await;
        if let Some(x) = read.as_ref() {
            x.verify()
        } else {
            true
        }
    }

    pub async fn reset(&self) {
        if let Some(x) = self.get_feature_mock.write().await.as_mut() {
            x.reset()
        }
    }

    pub async fn listening_address(&self) -> Option<String> {
        let r = self.server_handle.read().await;
        r.as_ref().map(|x| format!("http://{}", x.addr))
    }
}

#[tonic::async_trait]
impl RouteGuide for MockRouteGuideService {
    async fn get_feature(&self, request: Request<Point>) -> Result<Response<Feature>, Status> {
        info!("Got request");
        if let Some(s) = self.get_feature_mock.read().await.as_ref() {
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
