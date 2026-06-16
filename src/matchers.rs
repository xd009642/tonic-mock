use crate::{MatchResult, Matcher};
use tonic::metadata::{MetadataKey, MetadataValue};
use tonic::Request;

pub struct MetadataExistsMatcher {
    key: MetadataKey<tonic::metadata::Ascii>,
}

pub fn metadata_exists(name: impl AsRef<str>) -> MetadataExistsMatcher {
    MetadataExistsMatcher::new(name)
}

impl MetadataExistsMatcher {
    pub fn new(name: impl AsRef<str>) -> Self {
        Self {
            key: name.as_ref().parse().expect("invalid metadata key"),
        }
    }
}

impl<T> Matcher<T> for MetadataExistsMatcher {
    fn matches(&self, request: &Request<T>) -> MatchResult {
        if request.metadata().contains_key(&self.key) {
            MatchResult::Match
        } else {
            MatchResult::Mismatch
        }
    }
}

pub struct MetadataExactMatcher {
    key: MetadataKey<tonic::metadata::Ascii>,
    value: MetadataValue<tonic::metadata::Ascii>,
}

pub fn metadata_eq(name: impl AsRef<str>, value: impl AsRef<str>) -> MetadataExactMatcher {
    MetadataExactMatcher::new(name, value)
}

impl MetadataExactMatcher {
    pub fn new(name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        Self {
            key: name.as_ref().parse().expect("invalid metadata key"),
            value: value.as_ref().parse().expect("invalid metadata value"),
        }
    }
}

impl<T> Matcher<T> for MetadataExactMatcher {
    fn matches(&self, request: &Request<T>) -> MatchResult {
        if request.metadata().get(&self.key) == Some(&self.value) {
            MatchResult::Match
        } else {
            MatchResult::Mismatch
        }
    }
}

pub struct RequestMatcher<T> {
    matcher: Box<dyn Fn(&T) -> bool + Send + Sync + 'static>,
}

pub fn request_matches<T, F>(matcher: F) -> RequestMatcher<T>
where
    F: Fn(&T) -> bool + Send + Sync + 'static,
{
    RequestMatcher {
        matcher: Box::new(matcher),
    }
}

impl<T> Matcher<T> for RequestMatcher<T> {
    fn matches(&self, request: &Request<T>) -> MatchResult {
        if (self.matcher)(request.get_ref()) {
            MatchResult::Match
        } else {
            MatchResult::Mismatch
        }
    }
}
