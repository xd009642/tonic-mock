use crate::responder::*;
use crate::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tonic::metadata::MetadataKey;
use tonic::{Request, Response, Status, Streaming};

pub enum RequestType {
    Unary,
    ClientStream,
    ServerStream,
    BidirStream,
}

pub struct UnaryMethodMock<T, U> {
    matchers: Vec<Match<T>>,
    response: Box<dyn Responder<T, U> + Send + Sync>,
    called: AtomicUsize,
    expected_calls: Option<usize>,
    all_matched: AtomicBool,
}

impl<T, U> Default for UnaryMethodMock<T, U> {
    fn default() -> Self {
        Self {
            matchers: vec![],
            response: Box::new(Unimplemented),
            expected_calls: None,
            called: AtomicUsize::new(0),
            all_matched: AtomicBool::new(true),
        }
    }
}

impl<T, U> UnaryMethodMock<T, U> {
    pub fn process_request(&self, request: Request<T>) -> Result<Response<U>, Status> {
        self.called.fetch_add(1, Ordering::SeqCst);
        let mut matches = true;
        for matcher in &self.matchers {
            matches &= matcher.matches(&request);
            if !matches {
                self.all_matched.fetch_and(false, Ordering::SeqCst);
                // If we've failed matching do we want to send the response back?
                break;
            }
        }
        self.response.respond(request)
    }

    pub fn add_matcher(&mut self, m: impl Matcher<T> + Send + Sync + 'static) -> &mut Self {
        self.matchers.push(Match {
            matcher: Box::new(m),
        });
        self
    }

    pub fn response(&mut self, r: impl Responder<T, U> + Send + Sync + 'static) -> &mut Self {
        self.response = Box::new(r);
        self
    }

    pub fn expected(&mut self, calls: usize) -> &mut Self {
        self.expected_calls = Some(calls);
        self
    }

    pub fn verify(&self) -> bool {
        if let Some(calls) = self.expected_calls {
            let actual = self.called.load(Ordering::Relaxed);
            if actual != calls {
                return false;
            }
        }
        self.all_matched.load(Ordering::Relaxed)
    }
}

pub struct ClientStreamMethodMock<T, U> {
    matchers: Vec<Match<T>>,
    response: Box<dyn Responder<T, U> + Send + Sync>,
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
    matcher: Box<dyn Matcher<T> + Send + Sync>,
}

impl<T> Match<T> {
    pub fn matches(&self, request: &Request<T>) -> bool {
        self.matcher.matches(request)
    }
}

impl<T> From<Box<dyn Matcher<T> + Send + Sync>> for Match<T> {
    fn from(matcher: Box<dyn Matcher<T> + Send + Sync>) -> Self {
        Self { matcher }
    }
}

pub struct MetadataExistsMatcher(String);

impl MetadataExistsMatcher {
    pub fn new(key: String) -> Self {
        Self(key)
    }
}

impl<T> Matcher<T> for MetadataExistsMatcher {
    fn matches(&self, request: &Request<T>) -> bool {
        request.metadata().contains_key(&self.0)
    }
}
