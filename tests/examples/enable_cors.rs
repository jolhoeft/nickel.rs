use crate::util::{read_body_to_string, response_for_method, run_example};
use http::Method;

#[test]
fn sets_headers() {
    run_example("enable_cors", |port| {
        let paths = ["", "foo", "bar.html", "foo-barrrr/baz"];

        for path in &paths {
            let url = format!("http://localhost:{}/{}", port, path);
            let res = response_for_method(Method::GET, &url);

            assert_eq!(
                res.headers()
                    .get("access-control-allow-origin")
                    .map(|v| v.to_str().unwrap()),
                Some("*")
            );

            let allow_headers = res
                .headers()
                .get("access-control-allow-headers")
                .map(|v| v.to_str().unwrap().to_string());
            assert!(
                allow_headers.is_some(),
                "Missing access-control-allow-headers"
            );
            let allow_headers = allow_headers.unwrap();
            assert!(
                allow_headers.contains("Origin"),
                "Missing Origin in {}",
                allow_headers
            );
            assert!(
                allow_headers.contains("Content-Type"),
                "Missing Content-Type in {}",
                allow_headers
            );

            let body = read_body_to_string(res);
            assert_eq!(body, "Hello CORS Enabled World");
        }
    })
}
