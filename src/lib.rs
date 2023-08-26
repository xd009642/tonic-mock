use core::future::Future;
use http::header::HeaderMap;
use tonic::codec::Codec;
use tonic::{Request, Response, Status};

// Public macro reexport
pub use tonic_mock_macros::mock;

pub mod checker;
pub mod server;

pub mod prelude {
    pub use crate::checker::*;
    pub use crate::server::*;
}
