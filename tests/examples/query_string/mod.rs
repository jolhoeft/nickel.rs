use crate::util::*;

fn with_path<F>(path: &str, f: F)
where
    F: FnOnce(reqwest::blocking::Response),
{
    run_example("query_string", |port| {
        let url = format!("http://localhost:{}{}", port, path);
        let res = response_for(&url);
        f(res)
    })
}

mod all;
mod get;
