use crate::util::*;
use http::Method;

fn with_paths_and_method<F, S>(paths: &[S], method: Method, f: F)
where
    F: Fn(reqwest::blocking::Response),
    S: AsRef<str>,
{
    for path in paths {
        let method = method.clone();

        run_example("chaining", |port| {
            let url = format!("http://localhost:{}{}", port, path.as_ref());
            let res = response_for_method(method, &url);
            f(res)
        })
    }
}

mod expect_200 {
    use super::with_paths_and_method;
    use crate::util::*;
    use http::Method;

    #[test]
    fn root() {
        with_paths_and_method(&["/"], Method::GET, |res| {
            let s = read_body_to_string(res);
            assert_eq!(s, "Hello World");
        });
    }

    macro_rules! test {
        ($($method:ident),+) => (
            $(
                #[test]
                fn $method() {
                    let path = concat!("/", stringify!($method));
                    let method_str = stringify!($method);
                    let method: Method = method_str.to_uppercase().parse().unwrap();

                    println!("Method {:?}", method);
                    with_paths_and_method(&[path], method, |res| {
                        let s = read_body_to_string(res);
                        assert_eq!(s, method_str);
                    });
                }
            )+
        )
    }

    test!(get, post, put, patch, delete);
}

mod expect_404 {
    use super::with_paths_and_method;
    use http::Method;
    use http::StatusCode;

    static TEST_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE"];

    #[test]
    fn root() {
        for method_str in TEST_METHODS.iter().filter(|m| **m != "GET") {
            let method: Method = method_str.parse().unwrap();
            with_paths_and_method(&["/"], method, |res| {
                assert_eq!(res.status(), StatusCode::NOT_FOUND);
            })
        }
    }

    macro_rules! test {
        ($($method:ident),+) => (
            $(
                #[test]
                fn $method() {
                    let method_str = stringify!($method);
                    let method: Method = method_str.to_uppercase().parse().unwrap();

                    let paths = TEST_METHODS.iter()
                                            .filter(|m| m.to_lowercase() != method_str)
                                            .map(|m| format!("/{}", m.to_lowercase()))
                                            .collect::<Vec<_>>();

                    with_paths_and_method(&paths, method, |res| {
                        assert_eq!(res.status(), StatusCode::NOT_FOUND);
                    })
                }
            )+
        )
    }

    test!(get, post, put, patch, delete);
}
