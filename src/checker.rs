use crate::responder::*;
use crate::times::*;
use crate::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    called: AtomicU64,
    expected_calls: Option<Times>,
    all_matched: AtomicBool,
}

impl<T, U> Default for UnaryMethodMock<T, U> {
    fn default() -> Self {
        Self {
            matchers: vec![],
            response: Box::new(Unimplemented),
            expected_calls: None,
            called: AtomicU64::new(0),
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

    pub fn expect(&mut self, calls: impl Into<Times>) -> &mut Self {
        self.expected_calls = Some(calls.into());
        self
    }

    pub fn reset(&mut self) {
        self.called.store(0, Ordering::SeqCst);
        self.all_matched.store(true, Ordering::SeqCst);
    }

    pub fn verify(&self) -> bool {
        if let Some(calls) = self.expected_calls.as_ref() {
            let actual = self.called.load(Ordering::Relaxed);
            if !calls.contains(actual) {
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

pub(crate) enum MetadataLocation {
    Any,
    Header,
    Trailer,
}

pub struct MetadataExistsMatcher {
    key: String,
    area: MetadataLocation,
}

impl MetadataExistsMatcher {
    pub fn new(key: String) -> Self {
        Self {
            key,
            area: MetadataLocation::Any,
        }
    }

    pub fn header(key: String) -> Self {
        Self {
            key,
            area: MetadataLocation::Header,
        }
    }

    pub fn trailer(key: String) -> Self {
        Self {
            key,
            area: MetadataLocation::Trailer,
        }
    }
}

impl<T> Matcher<T> for MetadataExistsMatcher {
    fn matches(&self, request: &Request<T>) -> bool {
        request.metadata().contains_key(&self.key)
    }
}
