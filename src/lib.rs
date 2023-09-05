use core::future::Future;
use http::header::HeaderMap;
use tokio::sync::broadcast::{self, error::RecvError};
use tonic::codec::Codec;
use tonic::metadata::MetadataMap;
use tonic::{Request, Response, Status};
use tracing::error;

// Public macro reexport
pub use tonic_mock_macros::mock;

pub mod checker;
pub mod responder;
pub mod server;
pub mod times;

pub mod prelude {
    pub use crate::checker::*;
    pub use crate::responder::*;
    pub use crate::server::*;
}

pub trait Matcher<T> {
    fn matches(&self, request: &Request<T>) -> bool;
}

#[async_trait::async_trait]
pub trait StreamingMatcher<T: Clone + Send + 'static> {
    fn header_matches(&self, metadata: &MetadataMap, is_trailer: bool) -> bool {
        true
    }

    fn single_match(&self, value: &T) -> bool {
        true
    }

    async fn stream_match(&self, mut rx: broadcast::Receiver<Option<T>>) -> bool {
        let mut ret = true;
        let mut running = true;
        while running {
            let value = rx.recv().await;
            ret &= match value {
                Ok(Some(s)) => self.single_match(&s),
                Ok(None) | Err(RecvError::Closed) => {
                    running = false;
                    true
                }
                Err(RecvError::Lagged(lag)) => {
                    running = false;
                    error!("Checking channel lagged by {} messages", lag);
                    false
                }
            };
        }
        ret
    }
}

pub trait Responder<T, U> {
    fn respond(&self, request: Request<T>) -> Result<Response<U>, Status> {
        Err(tonic::Status::unimplemented("Method is not implemented"))
    }
}

#[async_trait::async_trait]
pub trait AsyncResponder<T, U> {
    async fn respond(&self, request: Request<T>) -> Result<Response<U>, Status>
    where
        T: Send + 'static,
    {
        Err(tonic::Status::unimplemented("Method is not implemented"))
    }
}

#[async_trait::async_trait]
impl<R, T, U> AsyncResponder<T, U> for R
where
    T: Send + 'static,
    R: Responder<T, U> + Sync,
{
    async fn respond(&self, request: Request<T>) -> Result<Response<U>, Status> {
        Responder::respond(self, request)
    }
}
