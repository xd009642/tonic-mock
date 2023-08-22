use crate::Future;
use bytes::Bytes;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;
use futures_util::pin_mut;
use futures_util::TryStreamExt;
use prost::Message;
use tonic::body::BoxBody;
use tonic::codec::Codec;
use tonic::codec::{ProstCodec, Streaming};
use tonic::transport::Body;
use tonic::{Code, Status};
use tower::Service;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

trait IncomingMessage: Message + Send {}
trait OutgoingMessage: Message + Send + Default {}

pub type DynCodec =
    ProstCodec<Box<dyn IncomingMessage + 'static>, Box<dyn OutgoingMessage + 'static>>;

pub struct GrpcMockServer {
    unary_matchers: Vec<Box<dyn UnaryResponder>>,
}

impl Service<http::Request<Body>> for GrpcMockServer {
    type Response = http::Response<BoxBody>;
    type Error = Status;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        // Now we want to get the Uri check against our matchers and pass what we need into the
        // future. Maybe we want different futures for streaming vs non-streaming request

        let fut = async move {
            let mut codec = DynCodec::default();
            codec.decoder();

            let (parts, body) = req.into_parts();

            let stream = Streaming::new_request(codec.decoder(), body, None, None);

            pin_mut!(stream);

            let message = stream
                .try_next()
                .await?
                .ok_or_else(|| Status::new(Code::Internal, "Missing request message"))?;

            if let Some(trailers) = stream.trailers().await? {}
            todo!()
        };
        Box::pin(fut)
    }
}
