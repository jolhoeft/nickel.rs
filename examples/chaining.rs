#[macro_use]
extern crate nickel;

use nickel::http::Method;
use nickel::{HttpRouter, Nickel};

fn main() {
    let mut server = Nickel::new();
    server
        .add_route(Method::GET, "/", middleware!("Hello World"))
        .get("/get", middleware!("get"))
        .post("/post", middleware!("post"))
        .put("/put", middleware!("put"))
        .patch("/patch", middleware!("patch"))
        .delete("/delete", middleware!("delete"));

    server.listen("127.0.0.1:6767").unwrap();
}
