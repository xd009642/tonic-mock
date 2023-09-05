use core::future::Future;
use http::header::HeaderMap;
use tonic::codec::Codec;
use tonic::{Request, Response, Status};

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
