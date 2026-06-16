pub mod matchers;
pub mod mock;
pub mod responder;
pub mod server;
pub mod times;

pub use crate::matchers::*;
pub use crate::mock::*;
pub use crate::responder::*;
pub use crate::server::*;
pub use crate::times::*;

pub mod prelude {
    pub use crate::matchers::*;
    pub use crate::mock::*;
    pub use crate::responder::*;
    pub use crate::server::*;
    pub use crate::times::*;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MatchResult {
    Match,
    Mismatch,
    Ignore,
}

pub trait Matcher<T> {
    fn matches(&self, request: &tonic::Request<T>) -> MatchResult;
}

impl<T, F> Matcher<T> for F
where
    F: Fn(&tonic::Request<T>) -> bool,
{
    fn matches(&self, request: &tonic::Request<T>) -> MatchResult {
        if self(request) {
            MatchResult::Match
        } else {
            MatchResult::Mismatch
        }
    }
}
