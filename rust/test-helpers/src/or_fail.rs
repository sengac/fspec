//! Turning a missing value or an error into a test failure with `?`, so
//! tests return `TestResult` instead of calling `unwrap`, `expect` or
//! `panic!`.

use std::fmt::{self, Display};

/// The error a failing test returns.
pub type TestError = Box<dyn std::error::Error>;

/// What every test returns.
pub type TestResult = Result<(), TestError>;

/// The failure `or_fail` reports; converts into any boxed error.
#[derive(Debug)]
pub struct Failure(String);

impl Failure {
    /// A failure with `message` (TEST-042: the deadlock guard's report).
    pub(crate) fn new(message: String) -> Self {
        Self(message)
    }
}

impl Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Failure {}

/// `value.or_fail("what was expected")?` fails the test with that message
/// (and the error, for a `Result`).
pub trait OrFail<T> {
    fn or_fail(self, expected: &str) -> Result<T, Failure>;
}

impl<T> OrFail<T> for Option<T> {
    fn or_fail(self, expected: &str) -> Result<T, Failure> {
        self.ok_or_else(|| Failure(format!("{expected}: got None")))
    }
}

impl<T, E: Display> OrFail<T> for Result<T, E> {
    fn or_fail(self, expected: &str) -> Result<T, Failure> {
        self.map_err(|error| Failure(format!("{expected}: {error}")))
    }
}

/// `result.or_fail_err("expected an error")?` — the error of a `Result`
/// that must have failed.
pub trait OrFailErr<E> {
    fn or_fail_err(self, expected: &str) -> Result<E, Failure>;
}

impl<T, E> OrFailErr<E> for Result<T, E> {
    fn or_fail_err(self, expected: &str) -> Result<E, Failure> {
        match self {
            Ok(_) => Err(Failure(format!("{expected}: got Ok"))),
            Err(error) => Ok(error),
        }
    }
}
