use crate::router::RouteResult;
use http::Method;
use plugin::{Extensible, Pluggable};
use typemap::TypeMap;

/// A container for all the request data.
///
/// The lifetime `'mw` represents the lifetime of various bits of
/// middleware state within nickel. It can vary and get shorter.
///
/// The lifetime `'server` represents the lifetime of data internal to
/// the server. It is fixed and longer than `'mw`.
pub struct Request<'mw, 'server: 'mw, D: 'mw = ()> {
    _phantom: std::marker::PhantomData<&'server ()>,
    /// The HTTP method
    method: Method,
    /// The request URI path (including query string)
    uri: String,
    /// The request headers
    headers: http::HeaderMap,
    /// The request body (read once, stored as bytes)
    body: Vec<u8>,
    /// Route params extracted by the router
    pub route_result: Option<RouteResult<'mw, D>>,

    map: TypeMap,

    data: &'mw D,
}

impl<'mw, 'server, D> Request<'mw, 'server, D> {
    pub fn from_internal(
        method: Method,
        uri: String,
        headers: http::HeaderMap,
        body: Vec<u8>,
        data: &'mw D,
    ) -> Request<'mw, 'server, D> {
        Request {
            _phantom: std::marker::PhantomData,
            method,
            uri,
            headers,
            body,
            route_result: None,
            map: TypeMap::new(),
            data,
        }
    }

    pub fn param(&self, key: &str) -> Option<&str> {
        self.route_result.as_ref().unwrap().param(key)
    }

    pub fn path_without_query(&self) -> Option<&str> {
        Some(self.uri.splitn(2, '?').next().unwrap())
    }

    pub fn server_data(&self) -> &'mw D {
        self.data
    }

    /// Get the HTTP method
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Get the request URI (path + query string)
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Get the request headers
    pub fn headers(&self) -> &http::HeaderMap {
        &self.headers
    }

    /// Get the request body as bytes
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Get the request body as a string
    pub fn body_as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.body)
    }

    /// Get a mutable reference to the URI (for mount rewriting)
    pub(crate) fn uri_mut(&mut self) -> &mut String {
        &mut self.uri
    }

    /// Get a reference to the server data (for internal use)
    pub(crate) fn data_ref(&self) -> &'mw D {
        self.data
    }
}

impl<'mw, 'server, D> Extensible for Request<'mw, 'server, D> {
    fn extensions(&self) -> &TypeMap {
        &self.map
    }

    fn extensions_mut(&mut self) -> &mut TypeMap {
        &mut self.map
    }
}

impl<'mw, 'server, D> Pluggable for Request<'mw, 'server, D> {}
