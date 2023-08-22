use tonic::{Request, Response, Status};

pub enum RequestType {
    Unary,
    ClientStream,
    ServerStream,
    BidirStream,
}

pub struct UnaryMethodMock<T, U> {
    matchers: Vec<Matcher<T>>,
    response: Box<dyn Responder>,
}

impl<T, U> UnaryMethodMock<T, U> {
    pub fn process_request(&self, t: Request<T>) -> Result<Response<U>, Status> {
        for matcher in &self.matchers {
            assert!(matcher.matches(&req))
        }
        self.response(self.response.respond(t))
    }
}

pub struct Matcher {
    matcher: Box<dyn Matcher<T, U>>,
}

impl Matcher {
    pub fn matches(request: &Request<T>) -> bool {
        self.matcher.matches(request)
    }
}

pub trait Matcher<T> {
    fn matches(request: &Request<T>) -> bool;
}

pub trait Responder<T, U> {
    fn respond(response: Request<T>) -> Result<Response<U>, Status>;
}
