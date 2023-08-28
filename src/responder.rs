use crate::*;
use tonic::{Response, Status};

pub struct FixedResponse<U> {
    response: Result<U, Status>,
}

impl<T, U> Responder<T, U> for FixedResponse<U>
where
    U: Clone,
{
    fn respond(&self, request: Request<T>) -> Result<Response<U>, Status> {
        self.response.clone().map(|x| Response::new(x))
    }
}
