use crate::nickel_error::NickelError;
use crate::request::Request;
use crate::response::Response;
use log::{debug, warn};

pub use self::Action::{Continue, Halt};

pub type MiddlewareResult<'mw, D = ()> =
    Result<Action<Response<'mw, D>, Response<'mw, D>>, NickelError<'mw, D>>;

pub enum Action<T = (), U = ()> {
    Continue(T),
    Halt(U),
}

// the usage of + Send is weird here because what we really want is + Static
// but that's not possible as of today. We have to use + Send for now.
pub trait Middleware<D>: Send + 'static + Sync {
    fn invoke<'mw, 'conn>(
        &'mw self,
        _req: &mut Request<'mw, 'conn, D>,
        res: Response<'mw, D>,
    ) -> MiddlewareResult<'mw, D> {
        res.next_middleware()
    }
}

impl<T, D> Middleware<D> for T
where
    T: for<'r, 'mw, 'conn> Fn(
            &'r mut Request<'mw, 'conn, D>,
            Response<'mw, D>,
        ) -> MiddlewareResult<'mw, D>
        + Send
        + Sync
        + 'static,
{
    fn invoke<'mw, 'conn>(
        &'mw self,
        req: &mut Request<'mw, 'conn, D>,
        res: Response<'mw, D>,
    ) -> MiddlewareResult<'mw, D> {
        (*self)(req, res)
    }
}

pub trait ErrorHandler<D>: Send + 'static + Sync {
    fn handle_error(&self, err: &mut NickelError<D>, req: &mut Request<D>) -> Action;
}

impl<D: 'static> ErrorHandler<D> for fn(&mut NickelError<D>, &mut Request<D>) -> Action {
    fn handle_error(&self, err: &mut NickelError<D>, req: &mut Request<D>) -> Action {
        (*self)(err, req)
    }
}

pub struct MiddlewareStack<D = ()> {
    handlers: Vec<Box<dyn Middleware<D> + Send + Sync>>,
    error_handlers: Vec<Box<dyn ErrorHandler<D> + Send + Sync>>,
}

impl<D: 'static> MiddlewareStack<D> {
    pub fn add_middleware<T: Middleware<D>>(&mut self, handler: T) {
        self.handlers.push(Box::new(handler));
    }

    pub fn add_error_handler<T: ErrorHandler<D>>(&mut self, handler: T) {
        self.error_handlers.push(Box::new(handler));
    }

    /// Invoke the middleware stack and return the final response.
    pub fn invoke<'mw, 'conn>(
        &'mw self,
        mut req: Request<'mw, 'conn, D>,
        mut res: Response<'mw, D>,
    ) -> Response<'mw, D> {
        for handler in self.handlers.iter() {
            match handler.invoke(&mut req, res) {
                Ok(Halt(halted_res)) => {
                    debug!(
                        "{} {} {}",
                        req.method(),
                        req.path_without_query().unwrap_or(""),
                        halted_res.status()
                    );
                    return halted_res;
                }
                Ok(Continue(fresh)) => res = fresh,
                Err(mut err) => {
                    warn!(
                        "{} {} {:?} {:?}",
                        req.method(),
                        req.path_without_query().unwrap_or(""),
                        err.message,
                        err.stream.as_ref().map(|s| s.status())
                    );

                    for error_handler in self.error_handlers.iter().rev() {
                        if let Halt(()) = error_handler.handle_error(&mut err, &mut req) {
                            // Error handler halted - return the response from the error
                            if let Some(error_res) = err.stream {
                                return error_res;
                            } else {
                                // No response available - return a minimal 500
                                warn!("Error handler halted but no response stream available");
                                return Response::empty_error(req.data_ref());
                            }
                        }
                    }

                    warn!(
                        "Unhandled error: {} {} {:?} {:?}",
                        req.method(),
                        req.path_without_query().unwrap_or(""),
                        err.message,
                        err.stream.as_ref().map(|s| s.status())
                    );

                    // Return whatever response we have from the error, or a 500
                    if let Some(error_res) = err.stream {
                        return error_res;
                    } else {
                        return Response::empty_error(req.data_ref());
                    }
                }
            }
        }

        // If we get here, all middleware passed through with Continue
        // Return the last response (probably a 404 from the final middleware)
        res
    }

    pub fn new() -> MiddlewareStack<D> {
        MiddlewareStack {
            handlers: Vec::new(),
            error_handlers: Vec::new(),
        }
    }
}
