use tonic::{Request, Response, Status, Streaming};

pub enum RequestType {
    Unary,
    ClientStream,
    ServerStream,
    BidirStream,
}

pub struct UnaryMethodMock<T, U> {
    matchers: Vec<Match<T>>,
    response: Box<dyn Responder<T, U>>,
}

impl<T, U> UnaryMethodMock<T, U> {
    pub fn process_request(&self, request: Request<T>) -> Result<Response<U>, Status> {
        for matcher in &self.matchers {
            assert!(matcher.matches(&request))
        }
        self.response.respond(request)
    }
}

pub struct ClientStreamMethodMock<T, U> {
    matchers: Vec<Match<T>>,
    response: Box<dyn Responder<T, U>>,
}

impl<T, U> ClientStreamMethodMock<T, U> {
    pub async fn process_request(
        &self,
        request: Request<Streaming<T>>,
    ) -> Result<Response<U>, Status> {
        //    for matcher in &self.matchers {
        //        assert!(matcher.matches(&request))
        //    }
        //    self.response.respond(request)
        todo!()
    }
}

pub struct Match<T> {
    matcher: Box<dyn Matcher<T>>,
}

impl<T> Match<T> {
    pub fn matches(&self, request: &Request<T>) -> bool {
        self.matcher.matches(request)
    }
}

pub trait Matcher<T> {
    fn matches(&self, request: &Request<T>) -> bool;
}

pub trait Responder<T, U> {
    fn respond(&self, response: Request<T>) -> Result<Response<U>, Status>;
}
