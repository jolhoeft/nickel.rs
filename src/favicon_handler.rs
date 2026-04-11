use log::debug;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use http::header::{HeaderValue, ALLOW};
use http::Method;
use http::StatusCode;

use crate::middleware::{Middleware, MiddlewareResult};
use crate::mimes::MediaType;
use crate::request::Request;
use crate::response::Response;

pub struct FaviconHandler {
    icon: Vec<u8>,
    icon_path: PathBuf, // Is it useful to log where in-memory favicon came from every request?
}

impl<D> Middleware<D> for FaviconHandler {
    fn invoke<'a, 'server>(
        &'a self,
        req: &mut Request<'a, 'server, D>,
        res: Response<'a, D>,
    ) -> MiddlewareResult<'a, D> {
        if FaviconHandler::is_favicon_request(req) {
            self.handle_request(req, res)
        } else {
            res.next_middleware()
        }
    }
}

impl FaviconHandler {
    /// Create a new middleware to serve an /favicon.ico file from an in-memory cache.
    /// The file is only read from disk once when the server starts.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use nickel::{Nickel, FaviconHandler};
    /// let mut server = Nickel::new();
    ///
    /// server.utilize(FaviconHandler::new("/path/to/ico/file"));
    /// ```
    pub fn new<P: AsRef<Path>>(icon_path: P) -> FaviconHandler {
        let icon_path = icon_path.as_ref().to_path_buf();
        let mut icon = vec![];
        File::open(&icon_path)
            .unwrap()
            .read_to_end(&mut icon)
            .unwrap();

        FaviconHandler {
            // Fail when favicon cannot be read. Better error message though?
            icon,
            icon_path,
        }
    }

    #[inline]
    pub fn is_favicon_request<D>(req: &Request<D>) -> bool {
        req.path_without_query() == Some("/favicon.ico")
    }

    pub fn handle_request<'a, D>(
        &self,
        req: &Request<D>,
        mut res: Response<'a, D>,
    ) -> MiddlewareResult<'a, D> {
        match *req.method() {
            Method::GET | Method::HEAD => self.send_favicon(req, res),
            Method::OPTIONS => {
                res.set(StatusCode::OK);
                res.headers_mut()
                    .insert(ALLOW, HeaderValue::from_static("GET, HEAD, OPTIONS"));
                res.send("")
            }
            _ => {
                res.set(StatusCode::METHOD_NOT_ALLOWED);
                res.headers_mut()
                    .insert(ALLOW, HeaderValue::from_static("GET, HEAD, OPTIONS"));
                res.send("")
            }
        }
    }

    pub fn send_favicon<'a, D>(
        &self,
        req: &Request<D>,
        mut res: Response<'a, D>,
    ) -> MiddlewareResult<'a, D> {
        debug!("{:?} {:?}", req.method(), self.icon_path.display());
        res.set(MediaType::Ico);
        res.send(&*self.icon)
    }
}
