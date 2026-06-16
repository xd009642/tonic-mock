use crate::responder::{ServerStreamingResponder, UnaryResponder};
use crate::times::Times;
use crate::{MatchResult, Matcher};
use futures::stream::BoxStream;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tonic::{Request, Response, Status};

pub struct Mock;

impl Mock {
    pub fn given<T>(matcher: impl Matcher<T> + Send + Sync + 'static) -> MockBuilder<T> {
        MockBuilder {
            matchers: vec![Box::new(matcher)],
            expected_calls: None,
            name: None,
        }
    }
}

pub struct MockBuilder<T> {
    matchers: Vec<Box<dyn Matcher<T> + Send + Sync + 'static>>,
    expected_calls: Option<Times>,
    name: Option<String>,
}

impl<T> MockBuilder<T> {
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn add_matcher(mut self, matcher: impl Matcher<T> + Send + Sync + 'static) -> Self {
        self.matchers.push(Box::new(matcher));
        self
    }

    pub fn expect(mut self, calls: impl Into<Times>) -> Self {
        self.expected_calls = Some(calls.into());
        self
    }

    pub fn respond_with<U>(
        self,
        responder: impl UnaryResponder<T, U> + Send + Sync + 'static,
    ) -> UnaryMethodMock<T, U> {
        UnaryMethodMock {
            core: MethodMockCore::new(self.matchers, self.expected_calls, self.name),
            responder: Box::new(responder),
            _response: PhantomData,
        }
    }

    pub fn respond_with_stream<U>(
        self,
        responder: impl ServerStreamingResponder<T, U> + Send + Sync + 'static,
    ) -> ServerStreamingMethodMock<T, U> {
        ServerStreamingMethodMock {
            core: MethodMockCore::new(self.matchers, self.expected_calls, self.name),
            responder: Box::new(responder),
            _response: PhantomData,
        }
    }
}

pub struct UnaryMethodMock<T, U> {
    core: MethodMockCore<T>,
    responder: Box<dyn UnaryResponder<T, U> + Send + Sync + 'static>,
    _response: PhantomData<U>,
}

impl<T, U> UnaryMethodMock<T, U> {
    pub fn process_request(&self, request: Request<T>) -> Result<Response<U>, Status> {
        self.core.observe(&request);
        self.responder.respond(request)
    }

    pub fn reset(&mut self) {
        self.core.reset();
    }

    pub fn verify_method(&self, method: &str) -> Result<(), VerificationError> {
        self.core.verify(method)
    }
}

impl<T, U> std::fmt::Debug for UnaryMethodMock<T, U> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnaryMethodMock").finish_non_exhaustive()
    }
}

pub struct ServerStreamingMethodMock<T, U> {
    core: MethodMockCore<T>,
    responder: Box<dyn ServerStreamingResponder<T, U> + Send + Sync + 'static>,
    _response: PhantomData<U>,
}

impl<T, U> ServerStreamingMethodMock<T, U> {
    pub fn process_request(
        &self,
        request: Request<T>,
    ) -> Result<Response<BoxStream<'static, Result<U, Status>>>, Status> {
        self.core.observe(&request);
        self.responder.respond(request)
    }

    pub fn reset(&mut self) {
        self.core.reset();
    }

    pub fn verify_method(&self, method: &str) -> Result<(), VerificationError> {
        self.core.verify(method)
    }
}

impl<T, U> std::fmt::Debug for ServerStreamingMethodMock<T, U> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerStreamingMethodMock")
            .finish_non_exhaustive()
    }
}

struct MethodMockCore<T> {
    matchers: Vec<Box<dyn Matcher<T> + Send + Sync + 'static>>,
    expected_calls: Option<Times>,
    name: Option<String>,
    called: AtomicU64,
    all_matched: AtomicBool,
}

impl<T> MethodMockCore<T> {
    fn new(
        matchers: Vec<Box<dyn Matcher<T> + Send + Sync + 'static>>,
        expected_calls: Option<Times>,
        name: Option<String>,
    ) -> Self {
        Self {
            matchers,
            expected_calls,
            name,
            called: AtomicU64::new(0),
            all_matched: AtomicBool::new(true),
        }
    }

    fn observe(&self, request: &Request<T>) {
        let matched = self
            .matchers
            .iter()
            .map(|matcher| matcher.matches(request))
            .all(|result| matches!(result, MatchResult::Match | MatchResult::Ignore));

        if matched {
            self.called.fetch_add(1, Ordering::SeqCst);
        } else {
            self.all_matched.store(false, Ordering::SeqCst);
        }
    }

    fn reset(&mut self) {
        self.called.store(0, Ordering::SeqCst);
        self.all_matched.store(true, Ordering::SeqCst);
    }

    fn verify(&self, method: &str) -> Result<(), VerificationError> {
        let calls = self.called.load(Ordering::SeqCst);
        let mut errors = Vec::new();
        let label = self
            .name
            .as_ref()
            .map(|name| format!("{} ({})", method, name))
            .unwrap_or_else(|| method.to_owned());

        if let Some(expected) = &self.expected_calls {
            if !expected.contains(calls) {
                errors.push(format!(
                    "{} expected calls {}, got {}",
                    label, expected, calls
                ));
            }
        }

        if !self.all_matched.load(Ordering::SeqCst) {
            errors.push(format!(
                "{} received at least one mismatched request",
                label
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(VerificationError::new(errors))
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VerificationError {
    messages: Vec<String>,
}

impl VerificationError {
    pub fn new(messages: Vec<String>) -> Self {
        Self { messages }
    }

    pub fn messages(&self) -> &[String] {
        &self.messages
    }
}

impl Display for VerificationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.messages.is_empty() {
            return write!(f, "mock verification failed");
        }

        write!(f, "mock verification failed: {}", self.messages.join("; "))
    }
}

impl std::error::Error for VerificationError {}
