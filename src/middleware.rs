use crate::request::Request;
use crate::response::Response;
use crate::nickel_error::NickelError;

pub use self::Action::{Continue, Halt};

pub type MiddlewareResult<'mw, B, D= ()> = Result<Action<Response<'mw, B, D>>,
                                                  NickelError<'mw, D>>;

pub enum Action<T=()> {
    Continue(T),
    Halt(T)
}

// the usage of + Send is weird here because what we really want is + Static
// but that's not possible as of today. We have to use + Send for now.
pub trait Middleware<B, D>: Send + 'static + Sync {
    fn invoke<'mw, 'conn>(&'mw self, _req: &mut Request<'mw, 'conn, D>, res: Response<'mw, B, D>) -> MiddlewareResult<'mw, B, D> {
        res.next_middleware()
    }
}

impl<T, D> Middleware<D> for T where T: for<'r, 'mw, 'conn> Fn(&'r mut Request<'mw, 'conn, D>, Response<'mw, D>) -> MiddlewareResult<'mw, D> + Send + Sync + 'static {
    fn invoke<'mw, 'conn>(&'mw self, req: &mut Request<'mw, 'conn, D>, res: Response<'mw, D>) -> MiddlewareResult<'mw, D> {
        (*self)(req, res)
    }
}

pub trait ErrorHandler<D>: Send + 'static + Sync {
    fn handle_error(&self, _: &mut NickelError<'_, D>, _: &mut Request<'_, '_, D>) -> Action;
}

impl<D: 'static> ErrorHandler<D> for fn(&mut NickelError<'_, D>, &mut Request<'_, '_, D>) -> Action {
    fn handle_error(&self, err: &mut NickelError<'_, D>, req: &mut Request<'_, '_, D>) -> Action {
        (*self)(err, req)
    }
}

pub struct MiddlewareStack<D=()> {
    handlers: Vec<Box<dyn Middleware<D> + Send + Sync>>,
    error_handlers: Vec<Box<dyn ErrorHandler<D> + Send + Sync>>
}

impl<D: 'static> MiddlewareStack<D> {
    pub fn add_middleware<T: Middleware<D>> (&mut self, handler: T) {
        self.handlers.push(Box::new(handler));
    }

    pub fn add_error_handler<T: ErrorHandler<D>> (&mut self, handler: T) {
        self.error_handlers.push(Box::new(handler));
    }

    pub fn invoke<'mw, 'conn>(&'mw self, mut req: Request<'mw, 'conn, D>, mut res: Response<'mw, D>) {
        for handler in self.handlers.iter() {
            match handler.invoke(&mut req, res) {
                Ok(Halt(res)) => {
                    debug!("Halted {:?} {:?} {:?} {:?}",
                           req.origin.method,
                           req.origin.remote_addr,
                           req.origin.uri,
                           res.status());
                    let _ = res.end();
                    return
                }
                Ok(Continue(fresh)) => res = fresh,
                Err(mut err) => {
                    warn!("{:?} {:?} {:?} {:?} {:?}",
                          req.origin.method,
                          req.origin.remote_addr,
                          req.origin.uri,
                          err.message,
                          err.stream.as_ref().map(|s| s.status()));

                    for error_handler in self.error_handlers.iter().rev() {
                        if let Halt(()) = error_handler.handle_error(&mut err, &mut req) {
                            err.end();
                            return
                        }
                    }

                    warn!("Unhandled error: {:?} {:?} {:?} {:?} {:?}",
                          req.origin.method,
                          req.origin.remote_addr,
                          req.origin.uri,
                          err.message,
                          err.stream.map(|s| s.status()));
                    return
                }
            }
        }
    }

    pub fn new () -> MiddlewareStack<D> {
        MiddlewareStack{
            handlers: Vec::new(),
            error_handlers: Vec::new()
        }
    }
}
