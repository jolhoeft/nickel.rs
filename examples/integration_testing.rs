// This attribute being conditional is an implementation detail of the nickel
// test setup for testing examples. Usually, it should just require `#[macro_use]`.
#[cfg_attr(not(test), macro_use)]
extern crate nickel;

use nickel::status::StatusCode;
use nickel::{HttpRouter, JsonBody, ListeningServer, MiddlewareResult, Nickel, Request, Response};
use serde_derive::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::sync::atomic::{self, AtomicUsize};
use std::{env, str};

// Example trait to allow custom databases to be used within the integration tests
// to test different behaviours without touching a real database.
trait Database: Send + Sync + 'static {
    fn get_users(&self) -> Vec<String>;
}

impl Database for Vec<String> {
    fn get_users(&self) -> Vec<String> {
        self.clone()
    }
}

struct ServerData {
    hits: AtomicUsize,

    // FIXME: The `middleware` macro typehinting doesn't support hinting with
    // typeparams, i.e. the hint can't be `< ServerData<T> >` where T is a
    // typeparam from the enclosing function, e.g. `start_server`.
    //
    // To counter this limitation, we've boxed the trait so that specifying the
    // typeparam is unnecessary.
    database: Box<dyn Database>,
}

impl ServerData {
    fn hitcount(&self) -> usize {
        self.hits.load(atomic::Ordering::Relaxed)
    }

    fn log_hit(&self) -> usize {
        self.hits.fetch_add(1, atomic::Ordering::Relaxed)
    }

    fn get_users(&self) -> Vec<String> {
        self.database.get_users()
    }
}

fn main() {
    let port = env::var("PORT").map(|s| s.parse().unwrap()).unwrap_or(3000);
    let address = &format!("0.0.0.0:{}", port);
    let database = vec![];

    start_server(address, database).unwrap();
}

fn log_hit<'mw, 'conn>(
    _req: &mut Request<'mw, 'conn, ServerData>,
    res: Response<'mw, ServerData>,
) -> MiddlewareResult<'mw, ServerData> {
    res.data().log_hit();
    return res.next_middleware();
}

fn start_server<D>(
    address: &str,
    database: D,
) -> Result<ListeningServer, Box<dyn StdError + Send + Sync>>
where
    D: Database,
{
    let server_data = ServerData {
        hits: AtomicUsize::new(0),
        database: Box::new(database),
    };

    let mut server = Nickel::with_data(server_data);

    // Track all hits to the server
    server.utilize(log_hit);

    // Core server
    server.get("/", middleware!("Hello World"));

    server.get("/users", middleware! { |_, res| <ServerData>
        let users = res.data().get_users();

        serde_json::from_str::<serde_json::Value>(&format!(r#"{{ "users": {:?} }}"#, users)).unwrap()
    });

    // Json example
    server.post("/", middleware! { |req, res|
        #[derive(Serialize, Deserialize)]
        struct Data { name: String, age: Option<u32> }

        let client = try_with!(res, req.json_as::<Data>().map_err(|e| (StatusCode::BAD_REQUEST, e)));

        match client.age {
            Some(age) => {
                serde_json::from_str::<serde_json::Value>(&format!(
                    r#"{{ "message": "Hello {}, your age is {}" }}"#,
                    client.name,
                    age
                )).unwrap()
            }
            None => {
                serde_json::from_str::<serde_json::Value>(&format!(
                    r#"{{ "message": "Hello {}, I don't know your age" }}"#,
                    client.name
                )).unwrap()
            }
        }
    });

    // Get the hitcount
    server.get(
        "/hits",
        middleware! { |_req, res| <ServerData>
            res.data().hitcount().to_string()
        },
    );

    server.listen(address)
}

#[cfg(test)]
mod tests {
    use self::support::{get, post, Server, STATIC_SERVER};

    use nickel::status::StatusCode;
    use serde_json::Value;

    use std::{thread, time};

    fn get_hits_after_delay(server: &Server) -> u32 {
        // let other tests hit the server
        thread::sleep(time::Duration::from_secs(1));

        let response = server.get("/hits");
        response.body().parse().unwrap()
    }

    #[test]
    fn root_responds_with_hello_world() {
        let response = get("/");

        assert_eq!(response.body(), "Hello World");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    // FIXME: This will probably fail if tests are run without parallelism
    fn server_is_shared_with_other_tests() {
        assert!(get_hits_after_delay(&STATIC_SERVER) > 1);
    }

    #[test]
    fn root_responds_with_modified_json() {
        let response = post("/", r#"{ "name": "Rust", "age": 1 }"#);

        let json: Value = serde_json::from_str(&response.body()).unwrap();

        assert_eq!(json["message"].as_str(), Some("Hello Rust, your age is 1"));
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.content_type_is_json());
    }

    #[test]
    fn accepts_json_with_missing_fields() {
        let response = post("/", r#"{ "name": "Rust" }"#);

        let json: Value = serde_json::from_str(&response.body()).unwrap();

        assert_eq!(
            json["message"].as_str(),
            Some("Hello Rust, I don't know your age")
        );
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.content_type_is_json());
    }

    #[test]
    fn doesnt_accept_bad_inputs() {
        let response = post("/", r#"{ }"#);
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// This test has a Server instance all to itself.
    #[test]
    fn non_shared_server() {
        let test_local_server = {
            let server = super::start_server("127.0.0.1:0", vec![]).unwrap();
            Server::new(server)
        };

        assert_eq!(get_hits_after_delay(&test_local_server), 1);
    }

    #[test]
    fn has_no_users_by_default() {
        let response = get("/users");

        let json: Value = serde_json::from_str(&response.body()).unwrap();

        assert_eq!(json["users"].as_array().unwrap().len(), 0);
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.content_type_is_json());
    }

    #[test]
    fn non_shared_server_with_different_database() {
        let server = {
            let bots = vec!["bors".into(), "homu".into(), "highfive".into()];
            let server = super::start_server("127.0.0.1:0", bots).unwrap();
            Server::new(server)
        };

        let response = server.get("/users");

        let json: Value = serde_json::from_str(&response.body()).unwrap();

        assert_eq!(json["users"].as_array().unwrap().len(), 3);
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.content_type_is_json());
    }

    mod support {
        use nickel::ListeningServer;
        use reqwest::blocking::{Client, Response as ReqwestResponse};

        use std::net::SocketAddr;

        pub trait Body {
            fn body(self) -> String;
        }

        impl Body for ReqwestResponse {
            fn body(self) -> String {
                let body = self.text().expect("Failed to read body of Response");
                println!("Read body: {}", body);
                body
            }
        }

        /// A wrapper around ReqwestResponse that provides access to status and body
        pub struct TestResponse {
            status: reqwest::StatusCode,
            content_type: Option<String>,
            body: String,
        }

        impl TestResponse {
            fn from_response(resp: ReqwestResponse) -> TestResponse {
                let status = resp.status();
                let content_type = resp
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let body = resp.text().expect("Failed to read body of Response");
                println!("Read body: {}", body);
                TestResponse {
                    status,
                    content_type,
                    body,
                }
            }

            pub fn body(&self) -> String {
                self.body.clone()
            }

            pub fn status(&self) -> nickel::http::StatusCode {
                nickel::http::StatusCode::from_u16(self.status.as_u16())
                    .unwrap_or(nickel::http::StatusCode::INTERNAL_SERVER_ERROR)
            }

            pub fn content_type_is_json(&self) -> bool {
                self.content_type
                    .as_ref()
                    .map(|ct| ct.contains("application/json"))
                    .unwrap_or(false)
            }
        }

        /// An example wrapper type to make testing more readable
        pub struct Server(SocketAddr);
        impl Server {
            pub fn new(server: ListeningServer) -> Server {
                let wrapped = Server(server.socket());

                // detaching is important otherwise it would block the test threads.
                server.detach();

                wrapped
            }

            pub fn get(&self, path: &str) -> TestResponse {
                let url = self.url_for(path);
                let resp = Client::new().get(&url).send().unwrap();
                TestResponse::from_response(resp)
            }

            pub fn post(&self, path: &str, body: &str) -> TestResponse {
                let url = self.url_for(path);
                let resp = Client::new()
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .body(body.to_string())
                    .send()
                    .unwrap();
                TestResponse::from_response(resp)
            }

            pub fn url_for(&self, path: &str) -> String {
                format!("http://{}{}", self.0, path)
            }
        }

        lazy_static::lazy_static! {
            /// This is a shared instance of the server between all the tests
            pub static ref STATIC_SERVER: Server = {
                let server = super::super::start_server("127.0.0.1:0", vec![]).unwrap();
                Server::new(server)
            };
        }

        /// Example of a free function version of `get` which uses the shared server
        pub fn get(path: &str) -> TestResponse {
            STATIC_SERVER.get(path)
        }

        /// Example of a free function version of `post` which uses the shared server
        pub fn post(path: &str, body: &str) -> TestResponse {
            STATIC_SERVER.post(path, body)
        }
    }
}
