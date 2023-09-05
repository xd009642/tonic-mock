//! This is taken from the internals of [wiremock](https://github.com/lukeMathWalker/wiremock-rs)
//! where I felt it was an appropriate abstraction to start from.
use std::fmt::{Debug, Formatter};
use std::ops::{
    Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};

#[derive(Clone, Debug)]
pub enum Times {
    Exact(u64),
    Unbounded(RangeFull),
    Range(Range<u64>),
    RangeFrom(RangeFrom<u64>),
    RangeTo(RangeTo<u64>),
    RangeToInclusive(RangeToInclusive<u64>),
    RangeInclusive(RangeInclusive<u64>),
}

impl Times {
    pub(crate) fn contains(&self, n_calls: u64) -> bool {
        match self {
            Times::Exact(e) => e == &n_calls,
            Times::Unbounded(r) => r.contains(&n_calls),
            Times::Range(r) => r.contains(&n_calls),
            Times::RangeFrom(r) => r.contains(&n_calls),
            Times::RangeTo(r) => r.contains(&n_calls),
            Times::RangeToInclusive(r) => r.contains(&n_calls),
            Times::RangeInclusive(r) => r.contains(&n_calls),
        }
    }
}

impl std::fmt::Display for Times {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Times::Exact(e) => write!(f, "== {}", e),
            Times::Unbounded(_) => write!(f, "0 <= x"),
            Times::Range(r) => write!(f, "{} <= x < {}", r.start, r.end),
            Times::RangeFrom(r) => write!(f, "{} <= x", r.start),
            Times::RangeTo(r) => write!(f, "0 <= x < {}", r.end),
            Times::RangeToInclusive(r) => write!(f, "0 <= x <= {}", r.end),
            Times::RangeInclusive(r) => write!(f, "{} <= x <= {}", r.start(), r.end()),
        }
    }
}

impl From<u64> for Times {
    fn from(x: u64) -> Self {
        Times::Exact(x)
    }
}

// A quick macro to help easing the implementation pain.
macro_rules! impl_from_for_range {
    ($type_name:ident) => {
        impl From<$type_name<u64>> for Times {
            fn from(r: $type_name<u64>) -> Self {
                Times::$type_name(r)
            }
        }
    };
}

impl_from_for_range!(Range);
impl_from_for_range!(RangeTo);
impl_from_for_range!(RangeFrom);
impl_from_for_range!(RangeInclusive);
impl_from_for_range!(RangeToInclusive);
