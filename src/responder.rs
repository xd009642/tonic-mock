use crate::*;
use tonic::{Response, Status};

pub struct FixedResponse<U> {
    response: Result<U, Status>,
}

impl<U> FixedResponse<U> {
    pub fn ok(u: U) -> Self {
        Self { response: Ok(u) }
    }

    pub fn err(s: Status) -> Self {
        Self { response: Err(s) }
    }
}

impl<U: Default> FixedResponse<U> {
    pub fn default_ok() -> Self {
        Self {
            response: Ok(Default::default()),
        }
    }
}

impl<T, U> Responder<T, U> for FixedResponse<U>
where
    U: Clone,
{
    fn respond(&self, request: Request<T>) -> Result<Response<U>, Status> {
        self.response.clone().map(|x| Response::new(x))
    }
}

pub struct Unimplemented;

impl<T, U> Responder<T, U> for Unimplemented {}
