use crate::responder::*;
use crate::times::*;
use crate::*;
use futures::stream::{FuturesOrdered, StreamExt};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::broadcast;
use tonic::metadata::MetadataKey;
use tonic::{Request, Response, Status, Streaming};
use tracing::trace;

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

pub struct ClientStreamMethodMock<T: Clone + Send + 'static, U> {
    matchers: Vec<StreamingMatch<T>>,
    response: Box<dyn Responder<T, U> + Send + Sync>,
}

impl<T, U> ClientStreamMethodMock<T, U>
where
    T: Clone + Send + 'static,
{
    pub async fn process_request(
        &self,
        request: Request<Streaming<T>>,
    ) -> Result<Response<U>, Status> {
        let mut matches = true;
        let metadata = request.metadata();
        let mut metadata_matches = vec![];

        for matcher in &self.matchers {
            metadata_matches.push(matcher.metadata_matches(metadata, false));
        }
        let mut matcher_results = FuturesOrdered::new();
        let (tx, rx) = broadcast::channel(32);

        for matcher in &self.matchers {
            matcher_results.push(matcher.stream_match(tx.subscribe()));
        }
        std::mem::drop(rx);

        let mut stream = request.into_inner();
        while let Ok(message) = stream.message().await {
            if let Err(e) = tx.send(message) {
                // If we fail to send to matchers it means none of the matchers cared about the
                // request body. Just metadata!
                trace!("Message broadcast failed, all matchers must be metadata matchers");
                break;
            }
        }

        std::mem::drop(tx);

        while let Some(checked) = matcher_results.next().await {
            matches &= checked;
        }

        let trailers = stream.trailers().await;
        match trailers {
            Ok(Some(map)) => {
                for (matcher, og) in self.matchers.iter().zip(metadata_matches.iter()) {
                    // TODO this is horribly wrong but oh well!
                    matches &= *og || matcher.metadata_matches(&map, true);
                }
            }
            _ => {
                for og in &metadata_matches {
                    matches &= *og;
                }
            }
        }

        //

        //    for matcher in &self.matchers {
        //        assert!(matcher.matches(&request))
        //    }
        //    self.response.respond(request)
        todo!()
    }
}

pub struct StreamingMatch<T: Clone + Send + 'static> {
    matcher: Box<dyn StreamingMatcher<T> + Send + Sync>,
}

impl<T: Clone + Send + 'static> StreamingMatch<T> {
    #[inline(always)]
    pub fn metadata_matches(&self, metadata: &MetadataMap, is_trailer: bool) -> bool {
        self.matcher.metadata_matches(metadata, is_trailer)
    }

    pub fn single_match(&self, value: &T) -> bool {
        unreachable!("single_match should not be called on the StreamingMatch wrapper type");
    }

    #[inline(always)]
    pub async fn stream_match(&self, rx: broadcast::Receiver<Option<T>>) -> bool {
        self.matcher.stream_match(rx).await
    }
}

pub struct Match<T> {
    matcher: Box<dyn Matcher<T> + Send + Sync>,
}

impl<T> Match<T> {
    #[inline(always)]
    pub fn matches(&self, request: &Request<T>) -> bool {
        self.matcher.matches(request)
    }
}

impl<T> From<Box<dyn Matcher<T> + Send + Sync>> for Match<T> {
    fn from(matcher: Box<dyn Matcher<T> + Send + Sync>) -> Self {
        Self { matcher }
    }
}

#[derive(Debug, Copy, Clone)]
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

#[async_trait::async_trait]
impl<T: Clone + Send + 'static> StreamingMatcher<T> for MetadataExistsMatcher {
    fn metadata_matches(&self, metadata: &MetadataMap, is_trailer: bool) -> bool {
        match (self.area, is_trailer) {
            (MetadataLocation::Any, _)
            | (MetadataLocation::Header, false)
            | (MetadataLocation::Trailer, true) => metadata.contains_key(&self.key),
            _ => true,
        }
    }

    async fn stream_match(&self, mut rx: broadcast::Receiver<Option<T>>) -> bool {
        true
    }
}
