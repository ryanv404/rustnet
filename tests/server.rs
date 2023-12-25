use std::process::{Command, Stdio};

use rustnet::{Body, Response};

#[macro_use]
mod common;
use common::{
    get_expected_for_client, get_expected_for_server, server_is_live,
};

mod a {
    use super::*;
    start_test_server!();
}

mod get {
    use super::*;
    run_test!(SERVER: GET get);
    run_test!(SERVER: GET about);
    run_test!(SERVER: GET foo);
    run_test!(SERVER: GET many_methods);
}

mod head {
    use super::*;
    run_test!(SERVER: HEAD head);
    run_test!(SERVER: HEAD get);
    run_test!(SERVER: HEAD foo);
    //run_test!(SERVER: HEAD favicon.ico);
    run_test!(SERVER: HEAD many_methods);
}

mod post {
    use super::*;
    run_test!(SERVER: POST post);
    run_test!(SERVER: POST foo);
    run_test!(SERVER: POST many_methods);
}

mod put {
    use super::*;
    run_test!(SERVER: PUT put);
    run_test!(SERVER: PUT foo);
    run_test!(SERVER: PUT many_methods);
}

mod patch {
    use super::*;
    run_test!(SERVER: PATCH patch);
    run_test!(SERVER: PATCH foo);
    run_test!(SERVER: PATCH many_methods);
}

mod delete {
    use super::*;
    run_test!(SERVER: DELETE delete);
    run_test!(SERVER: DELETE foo);
    run_test!(SERVER: DELETE many_methods);
}

mod trace {
    use super::*;
    run_test!(SERVER: TRACE trace);
    run_test!(SERVER: TRACE foo);
    run_test!(SERVER: TRACE many_methods);
}

mod options {
    use super::*;
    run_test!(SERVER: OPTIONS options);
    run_test!(SERVER: OPTIONS foo);
    run_test!(SERVER: OPTIONS many_methods);
}

mod connect {
    use super::*;
    run_test!(SERVER: CONNECT connect);
    run_test!(SERVER: CONNECT foo);
    run_test!(SERVER: CONNECT many_methods);
}

mod z {
    use super::*;
    shutdown_test_server!();
}
