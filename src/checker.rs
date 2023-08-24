use tonic::{Request, Response, Status};

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
