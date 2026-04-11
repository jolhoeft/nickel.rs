use crate::mimes::MediaType;
use crate::template_cache::TemplateCache;
use crate::{Action, Halt, MiddlewareResult, NickelError, Responder};
use http::header::{self, HeaderName, HeaderValue};
use http::StatusCode;
use modifier::Modifier;
use plugin::{Extensible, Pluggable};
use serde::Serialize;
use std::borrow::Cow;
use std::fs::File;
use std::io::{self, copy, Write};
use std::path::Path;
use typemap::TypeMap;

/// A container for the response.
///
/// In the new hyper 1.x architecture, we buffer the response body in memory
/// rather than using hyper's typestate (Fresh/Streaming). The response is
/// sent all at once when the middleware chain completes.
pub struct Response<'a, D: 'a = ()> {
    status: StatusCode,
    headers: http::HeaderMap,
    body: Vec<u8>,
    templates: &'a TemplateCache,
    data: &'a D,
    map: TypeMap,
    on_send: Vec<Box<dyn FnMut(&mut Response<'a, D>) + 'a>>,
    /// Whether start() has been called (body is being written)
    started: bool,
}

impl<'a, D> Response<'a, D> {
    pub fn from_internal(templates: &'a TemplateCache, data: &'a D) -> Response<'a, D> {
        Response {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: Vec::new(),
            templates,
            data,
            map: TypeMap::new(),
            on_send: vec![],
            started: false,
        }
    }

    /// Get a mutable reference to the status.
    pub fn status_mut(&mut self) -> &mut StatusCode {
        &mut self.status
    }

    /// Get a mutable reference to the Headers.
    pub fn headers_mut(&mut self) -> &mut http::HeaderMap {
        &mut self.headers
    }

    /// Modify the response with the provided data.
    ///
    /// # Examples
    /// ```rust,no_run
    /// #[macro_use] extern crate nickel;
    ///
    /// use nickel::{Nickel, HttpRouter, MediaType};
    /// use nickel::status::StatusCode;
    ///
    /// fn main() {
    ///     let mut server = Nickel::new();
    ///     server.get("/a", middleware! { |_, mut res|
    ///             // set the Status
    ///         res.set(StatusCode::PERMANENT_REDIRECT)
    ///             // update a Header value
    ///            .set_header("Location", "http://nickel.rs");
    ///
    ///         ""
    ///     });
    ///
    ///     server.get("/b", middleware! { |_, mut res|
    ///             // setting the content type
    ///         res.set(MediaType::Json);
    ///
    ///         "{'foo': 'bar'}"
    ///     });
    ///
    ///     // ...
    /// }
    /// ```
    pub fn set<T: Modifier<Response<'a, D>>>(&mut self, attribute: T) -> &mut Response<'a, D> {
        attribute.modify(self);
        self
    }

    /// Set a header by name and value.
    pub fn set_header<K, V>(&mut self, key: K, value: V) -> &mut Response<'a, D>
    where
        K: TryInto<HeaderName>,
        K::Error: std::fmt::Debug,
        V: TryInto<HeaderValue>,
        V::Error: std::fmt::Debug,
    {
        let name = key.try_into().expect("invalid header name");
        let val = value.try_into().expect("invalid header value");
        self.headers.insert(name, val);
        self
    }

    /// Writes a response
    ///
    /// # Examples
    /// ```rust,no_run
    /// use nickel::{Request, Response, MiddlewareResult};
    ///
    /// # #[allow(dead_code)]
    /// fn handler<'a, D>(_: &mut Request<D>, res: Response<'a, D>) -> MiddlewareResult<'a, D> {
    ///     res.send("hello world")
    /// }
    /// ```
    #[inline]
    pub fn send<T: Responder<D>>(self, data: T) -> MiddlewareResult<'a, D> {
        data.respond(self)
    }

    /// Writes a file to the output.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use nickel::{Request, Response, MiddlewareResult};
    /// use std::path::Path;
    ///
    /// # #[allow(dead_code)]
    /// fn handler<'a, D>(_: &mut Request<D>, res: Response<'a, D>) -> MiddlewareResult<'a, D> {
    ///     let favicon = Path::new("/assets/favicon.ico");
    ///     res.send_file(favicon)
    /// }
    /// ```
    pub fn send_file<P: AsRef<Path>>(mut self, path: P) -> MiddlewareResult<'a, D> {
        let path = path.as_ref();
        // Determine content type by file extension or default to binary
        let mime = mime_from_filename(path).unwrap_or(MediaType::Bin);
        self.set_header_fallback(header::CONTENT_TYPE, || {
            let mime_val: mime::Mime = mime.into();
            HeaderValue::from_str(&mime_val.to_string()).unwrap()
        });

        let mut file = try_with!(self, {
            File::open(path).map_err(|e| format!("Failed to send file '{:?}': {}", path, e))
        });

        self.start();
        match copy(&mut file, &mut self) {
            Ok(_) => Ok(Halt(self)),
            Err(e) => self.bail(format!("Failed to send file: {}", e)),
        }
    }

    // Set fallback headers
    fn set_fallback_headers(&mut self) {
        self.set_header_fallback(header::SERVER, || HeaderValue::from_static("Nickel"));
        self.set_header_fallback(header::CONTENT_TYPE, || {
            let mime_val: mime::Mime = MediaType::Html.into();
            HeaderValue::from_str(&mime_val.to_string()).unwrap()
        });
    }

    /// Return an error with the appropriate status code for error handlers to
    /// provide output for.
    pub fn error<T>(self, status: StatusCode, message: T) -> MiddlewareResult<'a, D>
    where
        T: Into<Cow<'static, str>>,
    {
        Err(NickelError::new(self, message, status))
    }

    /// Sets the header if not already set.
    ///
    /// If the header is not set then `f` will be called.
    pub fn set_header_fallback<F>(&mut self, key: HeaderName, f: F)
    where
        F: FnOnce() -> HeaderValue,
    {
        if !self.headers.contains_key(&key) {
            self.headers.insert(key, f());
        }
    }

    /// Renders the given template bound with the given data.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use std::collections::HashMap;
    /// use nickel::{Request, Response, MiddlewareResult};
    ///
    /// # #[allow(dead_code)]
    /// fn handler<'a, D>(_: &mut Request<D>, res: Response<'a, D>) -> MiddlewareResult<'a, D> {
    ///     let mut data = HashMap::new();
    ///     data.insert("name", "user");
    ///     res.render("examples/assets/template.tpl", &data)
    /// }
    /// ```
    pub fn render<T, P>(mut self, path: P, data: &T) -> MiddlewareResult<'a, D>
    where
        T: Serialize,
        P: AsRef<Path> + Into<String>,
    {
        self.start();
        match self.templates.render(path, &mut self, data) {
            Ok(()) => Ok(Halt(self)),
            Err(e) => self.bail(format!("Problem rendering template: {:?}", e)),
        }
    }

    /// Mark the response as started (triggers on_send hooks and fallback headers).
    pub fn start(&mut self) {
        if !self.started {
            self.started = true;

            // Run on_send hooks
            // We need to take the hooks out to avoid borrow issues
            let mut on_send = std::mem::take(&mut self.on_send);
            for f in on_send.iter_mut().rev() {
                f(self)
            }
            // Don't put them back - they've been consumed

            self.set_fallback_headers();
        }
    }

    pub fn server_data(&self) -> &'a D {
        self.data
    }

    pub fn on_send<F>(&mut self, f: F)
    where
        F: FnMut(&mut Response<'a, D>) + 'a,
    {
        self.on_send.push(Box::new(f))
    }

    /// Pass execution off to another Middleware
    ///
    /// When returned from a Middleware, it allows computation to continue
    /// in any Middleware queued after the active one.
    pub fn next_middleware(self) -> MiddlewareResult<'a, D> {
        Ok(Action::Continue(self))
    }

    /// In the case of an unrecoverable error while a stream is already in
    /// progress, there is no standard way to signal to the client that an
    /// error has occurred. `bail` will log an error message.
    pub fn bail<T>(self, message: T) -> MiddlewareResult<'a, D>
    where
        T: Into<Cow<'static, str>>,
    {
        unsafe { Err(NickelError::without_response(message)) }
    }

    /// The status of this response.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// The headers of this response.
    pub fn response_headers(&self) -> &http::HeaderMap {
        &self.headers
    }

    pub fn data(&self) -> &'a D {
        self.data
    }

    /// Create a minimal 500 error response (used when no response is available)
    pub(crate) fn empty_error(data: &'a D) -> Response<'a, D> {
        // We don't have templates available here, so we use a static reference trick
        // This is only used as a last resort when the middleware stack has no response
        use std::io::Write;
        static EMPTY_CACHE: std::sync::OnceLock<crate::template_cache::TemplateCache> =
            std::sync::OnceLock::new();
        let templates = EMPTY_CACHE.get_or_init(|| {
            crate::template_cache::TemplateCache::with_policy(
                crate::template_cache::ReloadPolicy::Never,
            )
        });
        let mut res = Response {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            headers: http::HeaderMap::new(),
            body: Vec::new(),
            templates,
            data,
            map: TypeMap::new(),
            on_send: vec![],
            started: true,
        };
        let _ = res.write_all(b"Internal Server Error");
        res
    }

    /// Consume this Response and produce the final http::Response for hyper.
    pub(crate) fn into_http_response(mut self) -> http::Response<Vec<u8>> {
        // Ensure start is called so fallback headers are set
        self.start();

        let mut builder = http::Response::builder().status(self.status);

        // Copy headers
        if let Some(headers) = builder.headers_mut() {
            *headers = self.headers;
        }

        builder.body(self.body).unwrap()
    }
}

impl<'a, D> Write for Response<'a, D> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.body.extend_from_slice(buf);
        Ok(buf.len())
    }

    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a, D> Extensible for Response<'a, D> {
    fn extensions(&self) -> &TypeMap {
        &self.map
    }

    fn extensions_mut(&mut self) -> &mut TypeMap {
        &mut self.map
    }
}

impl<'a, D> Pluggable for Response<'a, D> {}

fn mime_from_filename<P: AsRef<Path>>(path: P) -> Option<MediaType> {
    path.as_ref()
        .extension()
        .and_then(|os| os.to_str())
        // Lookup mime from file extension
        .and_then(|s| s.parse().ok())
}

#[test]
fn matches_content_type() {
    assert_eq!(Some(MediaType::Txt), mime_from_filename("test.txt"));
    assert_eq!(Some(MediaType::Json), mime_from_filename("test.json"));
    assert_eq!(Some(MediaType::Bin), mime_from_filename("test.bin"));
}

mod modifier_impls {
    use crate::{MediaType, Response};
    use http::header::{HeaderValue, CONTENT_TYPE};
    use http::StatusCode;
    use modifier::Modifier;

    impl<'a, D> Modifier<Response<'a, D>> for StatusCode {
        fn modify(self, res: &mut Response<'a, D>) {
            *res.status_mut() = self
        }
    }

    impl<'a, D> Modifier<Response<'a, D>> for MediaType {
        fn modify(self, res: &mut Response<'a, D>) {
            let mime_val: mime::Mime = self.into();
            if let Ok(val) = HeaderValue::from_str(&mime_val.to_string()) {
                res.headers_mut().insert(CONTENT_TYPE, val);
            }
        }
    }
}
