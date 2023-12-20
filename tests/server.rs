use std::process::{Command, Stdio};

#[macro_use]
mod common;

use common::{
    get_expected_server_output, get_server_test_output, server_is_live,
};

mod a {
    use super::*;
    run_server_tests!(START_TEST_SERVER);
}

mod serve {
    use super::*;

    run_server_tests! {
        get_routes:
        "GET", "/";
        "GET", "/foo";
    }

    run_server_tests! {
        head_routes:
        "HEAD", "/head";
        "HEAD", "/foo";
        "HEAD", "/favicon.ico";
    }

    run_server_tests! {
        post_routes:
        "POST", "/post";
        "POST", "/foo";
    }

    run_server_tests! {
        put_routes:
        "PUT", "/put";
        "PUT", "/foo";
    }

    run_server_tests! {
        patch_routes:
        "PATCH", "/patch";
        "PATCH", "/foo";
    }

    run_server_tests! {
        delete_routes:
        "DELETE", "/delete";
        "DELETE", "/foo";
    }

    run_server_tests! {
        trace_routes:
        "TRACE", "/trace";
        "TRACE", "/foo";
    }

    run_server_tests! {
        options_routes:
        "OPTIONS", "/options";
        "OPTIONS", "/foo";
    }

    run_server_tests! {
        connect_routes:
        "CONNECT", "/connect";
        "CONNECT", "/foo";
    }

    run_server_tests! {
        many_methods_one_path:
        "GET", "/many_methods";
        "HEAD", "/many_methods";
        "POST", "/many_methods";
        "PUT", "/many_methods";
        "PATCH", "/many_methods";
        "DELETE", "/many_methods";
        "TRACE", "/many_methods";
        "OPTIONS", "/many_methods";
        "CONNECT", "/many_methods";
    }
}

mod z {
    use super::*;
    run_server_tests!(SHUTDOWN_TEST_SERVER);
}
