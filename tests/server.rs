#[cfg(test)]
#[macro_use]
mod common;

// Use alphabetical module naming to start the test server first.
#[cfg(test)]
mod a {
    start_test_server!();
}

#[cfg(test)]
mod get {
    use super::*;

    run_test!(SERVER: GET known);
    run_test!(SERVER: GET unknown);
    run_test!(SERVER: GET many_methods);
}

#[cfg(test)]
mod head {
    use super::*;

    run_test!(SERVER: HEAD known);
    run_test!(SERVER: HEAD unknown);
    run_test!(SERVER: HEAD many_methods);
    //run_test!(SERVER: HEAD favicon.ico);
    // Confirm HEAD request to GET route succeeds.
    run_test!(SERVER: HEAD about);
}

#[cfg(test)]
mod post {
    use super::*;

    run_test!(SERVER: POST known);
    run_test!(SERVER: POST unknown);
    run_test!(SERVER: POST many_methods);
}

#[cfg(test)]
mod put {
    use super::*;

    run_test!(SERVER: PUT known);
    run_test!(SERVER: PUT unknown);
    run_test!(SERVER: PUT many_methods);
}

#[cfg(test)]
mod patch {
    use super::*;

    run_test!(SERVER: PATCH known);
    run_test!(SERVER: PATCH unknown);
    run_test!(SERVER: PATCH many_methods);
}

#[cfg(test)]
mod delete {
    use super::*;

    run_test!(SERVER: DELETE known);
    run_test!(SERVER: DELETE unknown);
    run_test!(SERVER: DELETE many_methods);
}

#[cfg(test)]
mod trace {
    use super::*;

    run_test!(SERVER: TRACE known);
    run_test!(SERVER: TRACE unknown);
    run_test!(SERVER: TRACE many_methods);
}

#[cfg(test)]
mod options {
    use super::*;

    run_test!(SERVER: OPTIONS known);
    run_test!(SERVER: OPTIONS unknown);
    run_test!(SERVER: OPTIONS many_methods);
}

#[cfg(test)]
mod connect {
    use super::*;

    run_test!(SERVER: CONNECT known);
    run_test!(SERVER: CONNECT unknown);
    run_test!(SERVER: CONNECT many_methods);
}

// Use alphabetical module naming to shut down the test server at the end.
#[cfg(test)]
mod z {
    shutdown_test_server!();
}
