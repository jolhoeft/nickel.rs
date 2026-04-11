//! A `Router` assigns `Middleware` to paths and resolves them per request
pub use self::http_router::HttpRouter;
pub use self::into_matcher::FORMAT_PARAM;
pub use self::matcher::Matcher;
pub use self::router::{Route, RouteResult, Router};

pub mod http_router;
mod into_matcher;
mod matcher;
pub mod router;
