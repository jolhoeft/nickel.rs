use crate::util::*;

fn with_path<F>(path: &str, f: F)
where
    F: FnOnce(reqwest::blocking::Response),
{
    run_example("json", |port| {
        let url = format!("http://localhost:{}{}", port, path);
        let res = response_for(&url);
        f(res)
    })
}

mod incoming {
    use crate::util::*;
    use http::StatusCode;

    fn send_body<F>(body: &str, f: F)
    where
        F: FnOnce(reqwest::blocking::Response),
    {
        run_example("json", |port| {
            let url = format!("http://localhost:{}", port);
            let res = response_for_post(&url, body);
            f(res)
        })
    }

    #[test]
    fn parses_valid_requests() {
        let body = r#"{ "first_name": "Beautiful", "last_name": "World" }"#;
        send_body(body, |res| {
            let s = read_body_to_string(res);
            assert_eq!(s, "Hello Beautiful World");
        })
    }

    #[test]
    fn rejects_invalid() {
        // Missing 'first_name'
        let body = r#"{ "lastname": "World" }"#;
        send_body(body, |res| {
            assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        })
    }
}

mod outgoing {
    mod to_json {
        use super::super::with_path;
        use crate::util::*;

        use std::collections::HashMap;

        #[test]
        fn serializes_valid_json() {
            with_path("/Pea/Nut", |res| {
                let s = read_body_to_string(res);
                let map: HashMap<String, String> = serde_json::from_str(&s).unwrap();
                assert_eq!(map["first_name"], "Pea");
                assert_eq!(map["last_name"], "Nut");
            })
        }

        #[test]
        fn sets_content_type_header() {
            with_path("/Pea/Nut", |res| {
                let content_type = res
                    .headers()
                    .get("content-type")
                    .map(|v| v.to_str().unwrap().to_string());
                assert!(
                    content_type
                        .as_ref()
                        .map(|s| s.contains("application/json"))
                        .unwrap_or(false),
                    "Expected application/json content type, got {:?}",
                    content_type
                );
            })
        }
    }

    mod raw {
        use super::super::with_path;
        use crate::util::*;

        use std::collections::HashMap;

        #[test]
        fn serializes_valid_json() {
            with_path("/raw", |res| {
                let s = read_body_to_string(res);
                let map: HashMap<String, String> = serde_json::from_str(&s).unwrap();
                assert_eq!(map["foo"], "bar");
            })
        }

        #[test]
        fn sets_content_type_header() {
            with_path("/raw", |res| {
                let content_type = res
                    .headers()
                    .get("content-type")
                    .map(|v| v.to_str().unwrap().to_string());
                assert!(
                    content_type
                        .as_ref()
                        .map(|s| s.contains("application/json"))
                        .unwrap_or(false),
                    "Expected application/json content type, got {:?}",
                    content_type
                );
            })
        }
    }
}
