use crate::util::*;
use http::StatusCode;

fn with_path<F>(path: &str, f: F)
where
    F: FnOnce(reqwest::blocking::Response),
{
    run_example("regex_route", |port| {
        let url = format!("http://localhost:{}{}", port, path);
        let res = response_for(&url);
        f(res)
    })
}

#[test]
fn with_param() {
    with_path("/hello/world", |res| {
        let s = read_body_to_string(res);
        assert_eq!(s, "Hello world");
    })
}

#[test]
fn ignores_query() {
    with_path("/hello/world?foo=bar", |res| {
        let s = read_body_to_string(res);
        assert_eq!(s, "Hello world");
    })
}

#[test]
fn fallthrough_too_many_params() {
    with_path("/hello/beautiful/world", |res| {
        let s = read_body_to_string(res);
        assert_eq!(s, "Hello beautiful");
    })
}

#[test]
fn fallthrough_with_no_match() {
    with_path("/", |res| {
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    })
}
