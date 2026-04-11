use crate::middleware::Middleware;
use crate::router::Matcher;
use http::Method;

pub trait HttpRouter<D> {
    /// Registers a handler to be used for a specified method.
    /// A handler can be anything implementing the `RequestHandler` trait.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// #[macro_use] extern crate nickel;
    /// extern crate regex;
    ///
    /// use nickel::{Nickel, HttpRouter};
    /// use http::Method;
    /// use regex::Regex;
    ///
    /// fn main() {
    ///     let mut server = Nickel::new();
    ///
    ///     server.add_route(Method::GET, "/foo", middleware! { "Get request! "});
    ///     server.add_route(Method::POST, "/foo", middleware! { |request|
    ///         format!("Method is: {}", request.method())
    ///     });
    ///     server.add_route(Method::PUT, "/foo", middleware! { |request|
    ///         format!("Method is: {}", request.method())
    ///     });
    ///     server.add_route(Method::DELETE, "/foo", middleware! { |request|
    ///         format!("Method is: {}", request.method())
    ///     });
    ///
    ///     // Regex path
    ///     let regex = Regex::new("/(foo|bar)").unwrap();
    ///     server.add_route(Method::GET, regex, middleware! { "Regex Get request! "});
    /// }
    /// ```
    fn add_route<M: Into<Matcher>, H: Middleware<D>>(
        &mut self,
        method: Method,
        matcher: M,
        handler: H,
    ) -> &mut Self;

    /// Registers a handler to be used for a specific GET request.
    /// Handlers are assigned to paths and paths are allowed to contain
    /// variables and wildcards.
    ///
    /// A handler added through this API will be attached to the default router.
    /// Consider creating the router middleware manually for advanced functionality.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// #[macro_use] extern crate nickel;
    /// use nickel::{Nickel, HttpRouter};
    ///
    /// fn main() {
    ///     let mut server = Nickel::new();
    ///
    ///     //  without variables or wildcards
    ///     server.get("/user", middleware! { "This matches /user" });
    ///
    ///     // with variables
    ///     server.get("/user/:userid", middleware! { |request|
    ///         format!("This is user: {}", request.param("userid").unwrap())
    ///     });
    ///
    ///     // with simple wildcard
    ///     server.get("/user/*/:userid", middleware! {
    ///         "This matches /user/list/4711 but not /user/extended/list/4711"
    ///     });
    ///
    ///     // with double wildcard
    ///     server.get("/user/**/:userid", middleware! {
    ///         "This matches /user/list/4711 and also /user/extended/list/4711"
    ///     });
    ///
    ///     // with chained routes
    ///     server
    ///         .get("/foo", middleware! {
    ///             "foo"
    ///         })
    ///         .get("/bar", middleware! {
    ///             "bar"
    ///         })
    ///         .get("/baz", middleware! {
    ///             "baz"
    ///         })
    ///         .get("/quux", middleware! {
    ///             "quux"
    ///         });
    /// }
    /// ```
    fn get<M: Into<Matcher>, H: Middleware<D>>(&mut self, matcher: M, handler: H) -> &mut Self {
        self.add_route(Method::GET, matcher, handler)
    }

    /// Registers a handler to be used for a specific HEAD request.
    ///
    /// Take a look at `get(...)` for a more detailed description.
    fn head<M: Into<Matcher>, H: Middleware<D>>(&mut self, matcher: M, handler: H) -> &mut Self {
        self.add_route(Method::HEAD, matcher, handler)
    }

    /// Registers a handler to be used for a specific POST request.
    ///
    /// Take a look at `get(...)` for a more detailed description.
    fn post<M: Into<Matcher>, H: Middleware<D>>(&mut self, matcher: M, handler: H) -> &mut Self {
        self.add_route(Method::POST, matcher, handler)
    }

    /// Registers a handler to be used for a specific PUT request.
    ///
    /// Take a look at `get(...)` for a more detailed description.
    fn put<M: Into<Matcher>, H: Middleware<D>>(&mut self, matcher: M, handler: H) -> &mut Self {
        self.add_route(Method::PUT, matcher, handler)
    }

    /// Registers a handler to be used for a specific DELETE request.
    ///
    /// Take a look at `get(...)` for a more detailed description.
    fn delete<M: Into<Matcher>, H: Middleware<D>>(&mut self, matcher: M, handler: H) -> &mut Self {
        self.add_route(Method::DELETE, matcher, handler)
    }

    /// Registers a handler to be used for a specific CONNECT request.
    ///
    /// Take a look at `get(...)` for a more detailed description.
    fn connect<M: Into<Matcher>, H: Middleware<D>>(&mut self, matcher: M, handler: H) -> &mut Self {
        self.add_route(Method::CONNECT, matcher, handler)
    }

    /// Registers a handler to be used for a specific OPTIONS request.
    ///
    /// Take a look at `get(...)` for a more detailed description.
    fn options<M: Into<Matcher>, H: Middleware<D>>(&mut self, matcher: M, handler: H) -> &mut Self {
        self.add_route(Method::OPTIONS, matcher, handler)
    }

    /// Registers a handler to be used for a specific TRACE request.
    ///
    /// Take a look at `get(...)` for a more detailed description.
    fn trace<M: Into<Matcher>, H: Middleware<D>>(&mut self, matcher: M, handler: H) -> &mut Self {
        self.add_route(Method::TRACE, matcher, handler)
    }

    /// Registers a handler to be used for a specific PATCH request.
    ///
    /// Take a look at `get(...)` for a more detailed description.
    fn patch<M: Into<Matcher>, H: Middleware<D>>(&mut self, matcher: M, handler: H) -> &mut Self {
        self.add_route(Method::PATCH, matcher, handler)
    }
}
