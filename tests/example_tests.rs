extern crate http;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
// HACK: integration_testing example refers to `nickel::foo`
// and this import helps that resolve rather than requiring `self::nickel::foo`
// which is an oddity due to the include method, which is used as tests in examples
// don't get executed otherwise.
#[macro_use]
extern crate nickel;

#[macro_use]
extern crate lazy_static;

mod util;

mod examples {
    mod chaining;
    mod custom_error_handler;
    mod enable_cors;
    mod form_data;
    mod hello_world;
    mod integration_testing;
    mod json;
    mod mount;
    mod moved_ownership;
    mod query_string;
    mod regex_route;
    mod route_data;
    mod routing;
    mod static_files;
    mod template;
}
