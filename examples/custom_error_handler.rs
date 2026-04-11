#[macro_use]
extern crate nickel;

use nickel::status::StatusCode;
use nickel::{Action, HttpRouter, Nickel, NickelError, Request};
use std::io::Write;

fn main() {
    let mut server = Nickel::new();

    // go to http://localhost:6767/user/4711 to see this route in action
    server.get(
        "/user/:userid",
        middleware! { |request|
            if let Some("42") = request.param("userid") {
                (StatusCode::OK, "User 42 was found!")
            } else {
                (StatusCode::IM_A_TEAPOT, "Teapot activated!")
            }
        },
    );

    //this is how to overwrite the default error handler to handle 404 cases with a custom view
    fn custom_handler<D>(err: &mut NickelError<D>, req: &mut Request<D>) -> Action {
        // Print the internal error message and path to the console
        println!(
            "[{}] ERROR: {}",
            req.path_without_query().unwrap(),
            err.message
        );

        if let Some(ref mut res) = err.stream {
            match res.status() {
                StatusCode::IM_A_TEAPOT => {
                    // Pass the internal message to the client
                    let _ = res.write_all(err.message.as_bytes());
                    return Action::Halt(());
                }
                StatusCode::NOT_FOUND => {
                    let _ = res.write_all(b"<h1>404 - Not Found</h1>");
                    return Action::Halt(());
                }
                _ => {}
            }
        }

        // Fall through to next error handler
        Action::Continue(())
    }

    // issue #20178
    let custom_handler: fn(&mut NickelError<()>, &mut Request<()>) -> Action = custom_handler;

    server.handle_error(custom_handler);

    server.listen("127.0.0.1:6767").unwrap();
}
