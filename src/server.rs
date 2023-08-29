use async_trait::async_trait;
use deadpool::managed::{Object, Pool};
use once_cell::sync::Lazy;
use std::convert::Infallible;
use tokio::net::TcpListener;
use tonic::body::BoxBody;
use tonic::transport::server::TcpIncoming;
use tonic::transport::Body;

static MOCK_SERVER_POOL: Lazy<Pool<MockServerPoolManager>> = Lazy::new(|| {
    Pool::builder(MockServerPoolManager)
        .max_size(1000)
        .build()
        .expect("Building a server pool is not expected to fail. Please report an issue")
});

pub(crate) type PooledMockServer = Object<MockServerPoolManager>;

async fn get_pooled_mock_server() -> PooledMockServer {
    MOCK_SERVER_POOL
        .get()
        .await
        .expect("Failed to get a GrpcMockServer from the pool")
}

pub struct MockServer {
    listener: TcpListener,
}

impl MockServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind an OS port for a mock server.");
        Self { listener }
    }
}

pub(crate) struct GrpcMockServer {
    listener: TcpIncoming,
}

impl GrpcMockServer {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind an OS port for a mock server.");

        // let local = listener.local_addr();

        todo!()
    }

    pub fn serve(&self) {}

    pub async fn reset(&self) {}
}

/// The `BareMockServer` pool manager.
///
/// It:
/// - creates a new `BareMockServer` if there is none to borrow from the pool;
/// - "cleans up" used `BareMockServer`s before making them available again for other tests to use.
pub(crate) struct MockServerPoolManager;

#[async_trait]
impl deadpool::managed::Manager for MockServerPoolManager {
    type Error = Infallible;
    type Type = GrpcMockServer;

    async fn create(&self) -> Result<GrpcMockServer, Infallible> {
        // All servers in the pool use the default configuration
        Ok(GrpcMockServer::start().await)
    }

    async fn recycle(
        &self,
        mock_server: &mut GrpcMockServer,
    ) -> deadpool::managed::RecycleResult<Infallible> {
        // Remove all existing settings - we want to start clean when the mock server
        // is picked up again from the pool.
        mock_server.reset().await;
        Ok(())
    }
}
