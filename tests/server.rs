use std::process::{Command, Stdio};

#[macro_use]
mod common;

use common::{
    get_expected_server_output, get_server_test_output,
    server_is_live,
};

mod a {
    use super::*;
    run_server_tests!(START_TEST_SERVER);
}

mod get {
    use super::*;
    run_server_tests! {
        known_route: "GET", "/";
        unknown_route: "GET", "/foo";
    }
}

mod head {
    use super::*;
    run_server_tests! {
        known_route: "HEAD", "/head";
        unknown_route: "HEAD", "/foo";
        favicon: "HEAD", "/favicon.ico";
    }
}

mod post {
    use super::*;
    run_server_tests! {
        known_route: "POST", "/post";
        unknown_route: "POST", "/foo";
    }
}

mod put {
    use super::*;
    run_server_tests! {
        known_route: "PUT", "/put";
        unknown_route: "PUT", "/foo";
    }
}

mod patch {
    use super::*;
    run_server_tests! {
        known_route: "PATCH", "/patch";
        unknown_route: "PATCH", "/foo";
    }
}

mod delete {
    use super::*;
    run_server_tests! {
        known_route: "DELETE", "/delete";
        unknown_route: "DELETE", "/foo";
    }
}

mod trace {
    use super::*;
    run_server_tests! {
        known_route: "TRACE", "/trace";
        unknown_route: "TRACE", "/foo";
    }
}

mod options {
    use super::*;
    run_server_tests! {
        known_route: "OPTIONS", "/options";
        unknown_route: "OPTIONS", "/foo";
    }
}

mod connect {
    use super::*;
    run_server_tests! {
        known_route: "CONNECT", "/connect";
        unknown_route: "CONNECT", "/foo";
    }
}

mod many_methods_same_path {
    use super::*;
    run_server_tests! {
        get: "GET", "/many_methods";
        head: "HEAD", "/many_methods";
        post: "POST", "/many_methods";
        put: "PUT", "/many_methods";
        patch: "PATCH", "/many_methods";
        delete: "DELETE", "/many_methods";
        trace: "TRACE", "/many_methods";
        options: "OPTIONS", "/many_methods";
        connect: "CONNECT", "/many_methods";
    }
}

mod z {
    use super::*;
    run_server_tests!(SHUTDOWN_TEST_SERVER);
}
