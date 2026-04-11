use crate::middleware::{Continue, Middleware, MiddlewareResult};
use crate::nickel::Nickel;
use crate::request::Request;
use crate::response::Response;

pub trait Mountable<D> {
    fn mount<S: Into<String>, M: Middleware<D>>(&mut self, mount_point: S, middleware: M);
}

impl<D> Mountable<D> for Nickel<D>
where
    D: Send + Sync + 'static,
{
    /// A trait that makes mounting more convenient. Works the same as
    /// manually adding a `Mount` middleware.
    ///
    ///
    /// # Examples
    /// ```rust,no_run
    /// use nickel::{Nickel, StaticFilesHandler, Mountable};
    /// let mut server = Nickel::new();
    ///
    /// server.mount("/static_files/", StaticFilesHandler::new("/path/to/serve/"));
    /// ```
    ///
    /// # Panics
    /// Panics if mount_point does not have a leading and trailing slash.
    fn mount<S: Into<String>, M: Middleware<D>>(&mut self, mount_point: S, middleware: M) {
        self.utilize(Mount::new(mount_point, middleware));
    }
}

pub struct Mount<M> {
    mount_point: String,
    middleware: M,
}

impl<M> Mount<M> {
    ///
    /// Creates a new middleware that mounts a middleware at a mount point.
    /// An incoming request that matches the mount point will be forwarded to
    /// the mounted middleware, but with the path rewritten so that the mount
    /// point appears to be the root from the perspective of the mounted
    /// middleware. This can be useful in combination with the
    /// `StaticFilesMiddleware`, for example.
    ///
    ///
    /// # Examples
    /// ```rust,no_run
    /// use nickel::{Nickel, StaticFilesHandler, Mount};
    /// let mut server = Nickel::new();
    ///
    /// server.utilize(
    ///     Mount::new("/static_files/",
    ///                StaticFilesHandler::new("/path/to/serve/")
    /// ));
    /// ```
    ///
    /// # Panics
    /// Panics if mount_point does not have a leading and trailing slash.
    pub fn new<S: Into<String>>(mount_point: S, middleware: M) -> Mount<M> {
        let mount_point: String = mount_point.into();
        match (mount_point.chars().last(), mount_point.chars().next()) {
            (Some('/'), Some('/')) => Mount {
                mount_point,
                middleware,
            },
            _ => panic!("Mount points must have a leading and trailing slash."),
        }
    }
}

impl<D, M: Middleware<D>> Middleware<D> for Mount<M> {
    fn invoke<'mw, 'conn>(
        &'mw self,
        req: &mut Request<'mw, 'conn, D>,
        res: Response<'mw, D>,
    ) -> MiddlewareResult<'mw, D> {
        let path = req.path_without_query().unwrap_or("").to_string();
        if path.starts_with(&self.mount_point) {
            let subpath = format!("/{}", &path[self.mount_point.len()..]);
            // Also preserve query string if present
            let original_uri = req.uri().to_string();
            let query = original_uri.splitn(2, '?').nth(1);
            let new_uri = match query {
                Some(q) => format!("{}?{}", subpath, q),
                None => subpath,
            };

            let original = std::mem::replace(req.uri_mut(), new_uri);
            let result = self.middleware.invoke(req, res);
            *req.uri_mut() = original;
            result
        } else {
            Ok(Continue(res))
        }
    }
}
