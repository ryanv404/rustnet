#[macro_use]
mod common;

mod a {
    start_test_server!();
}

mod get {
    use super::*;
    run_test!(SERVER: GET known);
    run_test!(SERVER: GET unknown);
    run_test!(SERVER: GET many_methods);
}

mod head {
    use super::*;
    run_test!(SERVER: HEAD known);
    run_test!(SERVER: HEAD unknown);
    run_test!(SERVER: HEAD many_methods);
    //run_test!(SERVER: HEAD favicon.ico);
    // Confirm HEAD request to GET route succeeds.
    run_test!(SERVER: HEAD about);
}

mod post {
    use super::*;
    run_test!(SERVER: POST known);
    run_test!(SERVER: POST unknown);
    run_test!(SERVER: POST many_methods);
}

mod put {
    use super::*;
    run_test!(SERVER: PUT known);
    run_test!(SERVER: PUT unknown);
    run_test!(SERVER: PUT many_methods);
}

mod patch {
    use super::*;
    run_test!(SERVER: PATCH known);
    run_test!(SERVER: PATCH unknown);
    run_test!(SERVER: PATCH many_methods);
}

mod delete {
    use super::*;
    run_test!(SERVER: DELETE known);
    run_test!(SERVER: DELETE unknown);
    run_test!(SERVER: DELETE many_methods);
}

mod trace {
    use super::*;
    run_test!(SERVER: TRACE known);
    run_test!(SERVER: TRACE unknown);
    run_test!(SERVER: TRACE many_methods);
}

mod options {
    use super::*;
    run_test!(SERVER: OPTIONS known);
    run_test!(SERVER: OPTIONS unknown);
    run_test!(SERVER: OPTIONS many_methods);
}

mod connect {
    use super::*;
    run_test!(SERVER: CONNECT known);
    run_test!(SERVER: CONNECT unknown);
    run_test!(SERVER: CONNECT many_methods);
}

mod z {
    shutdown_test_server!();
}
