use std::cmp::min;
use std::fmt::{self, Display};

use async_trait::async_trait;
use reqwest::Response;
use serde::{Deserialize, Deserializer};
use serde::de::DeserializeOwned;
use serde_json::Error as JsonError;
use serde_json::error::Category;

///
#[derive(Debug)]
pub enum Error {
    Serde(serde_json::Error),
    Span(SpanError),
}

#[derive(Debug)]
pub struct SpanError {
    span: String,
    idx: usize,
    error: JsonError,
}

impl SpanError {
    const PADDING: usize = 20;

    fn new(s: &str, error: JsonError) -> Self {
        // column is 1 based, but can be 0 sometimes
        let c = error.column().saturating_sub(1);
        let mut curlies = 0;
        let l = s[0..c].rfind(|c: char| match c {
            '{' => {
                curlies += 1;
                curlies >= 0
            }
            '}' => {
                curlies -= 1;
                false
            }
            _ => false,
        }).unwrap_or(0)
            .saturating_sub(Self::PADDING);
        let max = min(c + Self::PADDING, s.len());
        Self {
            span: s[l..max].to_string(),
            idx: c - l,
            error,
        }
    }
}

impl Display for SpanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let here = "^ here";
        write!(f,
               "{}\n{}\n{:->width$}",
               self.error,
               self.span,
               here,
               width = self.idx + here.len()
        )
    }
}

impl std::error::Error for SpanError {}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Serde(serde) => write!(f, "{}", serde),
            Self::Span(span) => write!(f, "{}", span),
        }
    }
}

impl std::error::Error for Error {}

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Serde(JsonError::custom(msg))
    }
}

// make stuff ambiguous so probably bad
// impl From<JsonError> for Error {
//     fn from(e: JsonError) -> Self {
//         Self::Serde(e)
//     }
// }
//
// impl From<SpanError> for Error {
//     fn from(span: SpanError) -> Self {
//         Self::Span(span)
//     }
// }

/// A wrapper for [`serde_json::from_str`](serde_json::from_str) that wraps parsing errors with
/// information showing where in [`s`](s) the error occurred.
///
/// # Errors
///
/// If [`serde_json::from_str`](serde_json::from_str) errors, and with more information
pub fn nice_from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T, Error> {
    match serde_json::from_str(s) {
        Ok(t) => Ok(t),
        Err(e) => {
            match e.classify() {
                Category::Syntax | Category::Data => {
                    let line = s.lines()
                        .nth(e.line().saturating_sub(1))
                        .unwrap();
                    Err(Error::Span(SpanError::new(line, e)))
                }
                Category::Eof | Category::Io => Err(Error::Serde(e))
            }
        }
    }
}

#[async_trait]
pub trait NiceResponseJson {
    async fn nice_json<T: DeserializeOwned>(self) -> crate::http::ClientResult<T>;
}

#[async_trait]
impl NiceResponseJson for Response {
    async fn nice_json<T: DeserializeOwned>(self) -> crate::http::ClientResult<T> {
        let mut text = self.text().await?;
        if text.is_empty() {
            text = "null".into();
        }
        Ok(nice_from_str(&text)?)
    }
}

pub trait BoolExt {
    fn default_true() -> bool { true }
    fn is_true(&self) -> bool;
    fn is_false(&self) -> bool;
}

impl BoolExt for bool {
    fn is_true(&self) -> bool { *self }

    fn is_false(&self) -> bool { !*self }
}

pub fn null_as_t<'de, D, T>(d: D) -> Result<T, D::Error>
    where D: Deserializer<'de>,
          T: Default,
          Option<T>: Deserialize<'de> {
    Ok(<Option<T>>::deserialize(d)?
        .unwrap_or_default())
}

/// All
pub trait SkipUnit {
    fn should_skip(&self) -> bool;
}

impl SkipUnit for () {
    fn should_skip(&self) -> bool {
        true
    }
}

impl<T> SkipUnit for Vec<T> {
    fn should_skip(&self) -> bool {
        self.is_empty()
    }
}