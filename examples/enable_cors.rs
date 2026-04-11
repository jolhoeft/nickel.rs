#[macro_use]
extern crate nickel;

use nickel::{HttpRouter, MiddlewareResult, Nickel, Request, Response};

fn enable_cors<'mw>(_req: &mut Request, mut res: Response<'mw>) -> MiddlewareResult<'mw> {
    // Set appropriate headers
    res.set_header("Access-Control-Allow-Origin", "*");
    res.set_header(
        "Access-Control-Allow-Headers",
        "Origin, X-Requested-With, Content-Type, Accept",
    );

    // Pass control to the next middleware
    res.next_middleware()
}

fn main() {
    let mut server = Nickel::new();
    server.utilize(enable_cors);
    server.options("**", middleware!(""));
    server.get("**", middleware!("Hello CORS Enabled World"));
    server.listen("127.0.0.1:6767").unwrap();
}
