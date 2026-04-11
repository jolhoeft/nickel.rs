use crate::request::Request;
use crate::status::StatusCode;
use crate::urlencoded::{self, Params};
use plugin::{Pluggable, Plugin};
use serde::Deserialize;
use serde_json;
use std::error::Error as StdError;
use std::fmt;
use std::io::{self, ErrorKind};
use typemap::Key;

struct BodyReader;

impl Key for BodyReader {
    type Value = String;
}

impl<'mw, 'conn, D> Plugin<Request<'mw, 'conn, D>> for BodyReader {
    type Error = io::Error;

    fn eval(req: &mut Request<D>) -> Result<String, io::Error> {
        match req.body_as_str() {
            Ok(s) => Ok(s.to_string()),
            Err(e) => Err(io::Error::new(ErrorKind::InvalidData, e)),
        }
    }
}

struct FormBodyParser;

impl Key for FormBodyParser {
    type Value = Params;
}

impl<'mw, 'conn, D> Plugin<Request<'mw, 'conn, D>> for FormBodyParser {
    type Error = BodyError;

    fn eval(req: &mut Request<D>) -> Result<Params, BodyError> {
        // Check content type
        let is_form = req
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.starts_with("application/x-www-form-urlencoded"))
            .unwrap_or(false);

        if is_form {
            let body = req.get_ref::<BodyReader>()?;
            Ok(urlencoded::parse(body))
        } else {
            Err(BodyError::WrongContentType)
        }
    }
}

pub trait FormBody {
    /// Extracts URL encoded data from the request body.
    /// # Examples
    /// ```rust,no_run
    /// #[macro_use] extern crate nickel;
    /// use nickel::{Nickel, HttpRouter, FormBody};
    ///
    /// fn main() {
    ///     let mut server = Nickel::new();
    ///     server.post("/a", middleware! { |req, res|
    ///         let form_body = try_with!(res, req.form_body());
    ///         format!("Post: {:?}", form_body)
    ///     });
    /// }
    /// ```
    fn form_body(&mut self) -> Result<&Params, (StatusCode, BodyError)>;
}

impl<'mw, 'conn, D> FormBody for Request<'mw, 'conn, D> {
    fn form_body(&mut self) -> Result<&Params, (StatusCode, BodyError)> {
        self.get_ref::<FormBodyParser>()
            .map_err(|e| (StatusCode::BAD_REQUEST, e))
    }
}

pub trait JsonBody {
    fn json_as<'a, T: Deserialize<'a>>(&'a mut self) -> Result<T, io::Error>;
}

impl<'mw, 'conn, D> JsonBody for Request<'mw, 'conn, D> {
    // FIXME: Update the error type.
    // Would be good to capture parsing error rather than a generic io::Error.
    // FIXME: Do the content-type check
    fn json_as<'a, T: Deserialize<'a>>(&'a mut self) -> Result<T, io::Error> {
        self.get_ref::<BodyReader>().and_then(|body| {
            serde_json::from_str::<T>(body)
                .map_err(|err| io::Error::new(ErrorKind::Other, format!("Parse error: {}", err)))
        })
    }
}

#[derive(Debug)]
pub enum BodyError {
    Io(io::Error),
    WrongContentType,
}

impl From<io::Error> for BodyError {
    fn from(err: io::Error) -> BodyError {
        BodyError::Io(err)
    }
}

impl StdError for BodyError {
    fn cause(&self) -> Option<&dyn StdError> {
        match *self {
            BodyError::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for BodyError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BodyError::Io(ref err) => write!(out, "{}", err),
            BodyError::WrongContentType => write!(out, "Wrong content type"),
        }
    }
}
