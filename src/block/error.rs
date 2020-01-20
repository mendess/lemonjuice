use std::{error::Error, fmt::{self, Display},borrow::Cow};

#[derive(Debug, Clone)]
pub struct ParseError<'a>(&'a str, Cow<'a, str>);

impl<'a> From<(&'a str, &'a str)> for ParseError<'a> {
    fn from(t: (&'a str, &'a str)) -> Self {
        Self(t.0, t.1.into())
    }
}

impl<'a> From<(&'a str, String)> for ParseError<'a> {
    fn from(e: (&'a str, String)) -> Self {
        Self(e.0, e.1.into())
    }
}

impl<'a> Display for ParseError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "tried to parse '{}' but {}", self.0, self.1)
    }
}

impl<'a> Error for ParseError<'a> {}
