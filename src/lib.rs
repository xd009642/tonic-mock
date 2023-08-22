use core::future::Future;
use http::header::HeaderMap;
use tonic::codec::Codec;
use tonic::{Request, Response, Status};

pub mod checker;
pub mod server;

pub trait UnaryResponder<T: Message + Send + 'static, U: Message + Send + Default + 'static> {
    fn response(headers: HeaderMap, value: T) -> Result<U, Status>;
}

pub trait ServerStreamResponder<T: Message + Send + 'static, U: Message + Send + Default + 'static>
{
    fn response(headers: HeaderMap, value: T) -> Result<U, Status>;
}
