//! Blanket impls for Middleware.
//! This is pre-implemented for any function which takes a
//! `Request` and `Response` parameter and returns anything
//! implementing the `Responder` trait. It is also
//! implemented for a tuple of a function and a type `T`.
//! The function must take a `Request`, a `Response` and a
//! `T`, returning anything that implements `Responder`.
//! The data of type `T` will then be shared and available
//! in any request.
//!
//! Please see the examples for usage.
use crate::mimes::MediaType;
use crate::{Halt, MiddlewareResult, NickelError, Response};
use http::header::{HeaderValue, CONTENT_TYPE};
use http::StatusCode;
use serde_json;
use std::io::Write;

/// This trait provides convenience for translating a number
/// of common return types into a `MiddlewareResult` while
/// also modifying the `Response` as required.
///
/// Please see the examples for some uses.
pub trait Responder<D> {
    fn respond<'a>(self, res: Response<'a, D>) -> MiddlewareResult<'a, D>;
}

impl<D> Responder<D> for () {
    fn respond<'a>(self, res: Response<'a, D>) -> MiddlewareResult<'a, D> {
        res.next_middleware()
    }
}

impl<D> Responder<D> for serde_json::Value {
    fn respond<'a>(self, mut res: Response<'a, D>) -> MiddlewareResult<'a, D> {
        maybe_set_type(&mut res, MediaType::Json);
        res.send(serde_json::to_string(&self).map_err(|e| format!("Failed to parse JSON: {}", e)))
    }
}

impl<T, E, D> Responder<D> for Result<T, E>
where
    T: Responder<D>,
    for<'e> NickelError<'e, D>: From<(Response<'e, D>, E)>,
{
    fn respond<'a>(self, res: Response<'a, D>) -> MiddlewareResult<'a, D> {
        let data = try_with!(res, self);
        res.send(data)
    }
}

macro_rules! dual_impl {
    ($view:ty, $alloc:ty, |$s:ident, $res:ident| $b:block) => (
        impl<'a, D> Responder<D> for $view {
            #[allow(unused_mut)]
            #[inline]
            fn respond<'c>($s, mut $res: Response<'c, D>) -> MiddlewareResult<'c, D> $b
        }

        impl<'a, D> Responder<D> for $alloc {
            #[allow(unused_mut)]
            #[inline]
            fn respond<'c>($s, mut $res: Response<'c, D>) -> MiddlewareResult<'c, D> $b
        }
    )
}

dual_impl!(&'a [u8], Vec<u8>, |self, res| {
    maybe_set_type(&mut res, MediaType::Bin);

    res.start();
    match res.write_all(&self[..]) {
        Ok(()) => Ok(Halt(res)),
        Err(e) => res.bail(format!("Failed to send: {}", e)),
    }
});

dual_impl!(&'a str, String, |self, res| {
    maybe_set_type(&mut res, MediaType::Html);
    res.send(self.as_bytes())
});

dual_impl!(
    (StatusCode, &'static str),
    (StatusCode, String),
    |self, res| {
        let (status, message) = self;

        if status.is_client_error() || status.is_server_error() {
            res.error(status, message)
        } else {
            res.set(status);
            res.send(message)
        }
    }
);

impl<'a, D> Responder<D> for StatusCode {
    #[inline]
    fn respond<'c>(self, res: Response<'c, D>) -> MiddlewareResult<'c, D> {
        res.send((self, ""))
    }
}

dual_impl!(&'a [&'a str], &'a [String], |self, res| {
    maybe_set_type(&mut res, MediaType::Html);

    res.start();
    for ref s in self.iter() {
        if let Err(e) = res.write_all(s.as_bytes()) {
            return res.bail(format!("Failed to write to stream: {}", e));
        }
    }
    Ok(Halt(res))
});

dual_impl!((u16, &'static str), (u16, String), |self, res| {
    let (status, message) = self;
    res.send((
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        message,
    ))
});

fn maybe_set_type<D>(res: &mut Response<D>, media_type: MediaType) {
    let mime_val: mime::Mime = media_type.into();
    res.set_header_fallback(CONTENT_TYPE, || {
        HeaderValue::from_str(&mime_val.to_string()).unwrap()
    });
}
