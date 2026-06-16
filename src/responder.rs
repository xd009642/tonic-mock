use futures::stream::{self, BoxStream};
use futures::Stream;
use futures::StreamExt;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub trait UnaryResponder<T, U> {
    fn respond(&self, request: Request<T>) -> Result<Response<U>, Status>;
}

pub trait ServerStreamingResponder<T, U> {
    fn respond(
        &self,
        request: Request<T>,
    ) -> Result<Response<BoxStream<'static, Result<U, Status>>>, Status>;
}

pub struct FixedResponse<U> {
    response: Result<U, Status>,
}

impl<U> FixedResponse<U> {
    pub fn ok(value: U) -> Self {
        Self {
            response: Ok(value),
        }
    }

    pub fn err(status: Status) -> Self {
        Self {
            response: Err(status),
        }
    }
}

impl<U: Default> FixedResponse<U> {
    pub fn default_ok() -> Self {
        Self::ok(U::default())
    }
}

impl<T, U> UnaryResponder<T, U> for FixedResponse<U>
where
    U: Clone,
{
    fn respond(&self, _request: Request<T>) -> Result<Response<U>, Status> {
        self.response.clone().map(Response::new)
    }
}

pub struct StreamResponse<U> {
    stream_ctor: Arc<dyn Fn() -> BoxStream<'static, Result<U, Status>> + Send + Sync + 'static>,
}

impl<U> StreamResponse<U> {
    pub fn from_stream<F, S>(stream_ctor: F) -> Self
    where
        F: Fn() -> S + Send + Sync + 'static,
        S: Stream<Item = Result<U, Status>> + Send + 'static,
    {
        Self {
            stream_ctor: Arc::new(move || stream_ctor().boxed()),
        }
    }
}

impl<U> StreamResponse<U>
where
    U: Clone + Send + Sync + 'static,
{
    pub fn from_iter(values: impl IntoIterator<Item = U>) -> Self {
        let values = Arc::new(values.into_iter().collect::<Vec<_>>());
        Self::from_stream(move || {
            let values = Arc::clone(&values);
            stream::iter(values.iter().cloned().map(Ok).collect::<Vec<_>>())
        })
    }
}

impl<T, U> ServerStreamingResponder<T, U> for StreamResponse<U>
where
    U: Send + 'static,
{
    fn respond(
        &self,
        _request: Request<T>,
    ) -> Result<Response<BoxStream<'static, Result<U, Status>>>, Status> {
        Ok(Response::new((self.stream_ctor)()))
    }
}

pub struct StatusResponse {
    status: Status,
}

impl StatusResponse {
    pub fn new(status: Status) -> Self {
        Self { status }
    }
}

impl<T, U> ServerStreamingResponder<T, U> for StatusResponse {
    fn respond(
        &self,
        _request: Request<T>,
    ) -> Result<Response<BoxStream<'static, Result<U, Status>>>, Status> {
        Err(Status::new(
            self.status.code(),
            self.status.message().to_owned(),
        ))
    }
}
