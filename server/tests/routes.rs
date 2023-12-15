#[macro_use]
mod common;

mod a {
    run_server_tests!(START_TEST_SERVER);
}

mod get {
    run_server_tests! {
        about: "GET", "/about";
        foo: "GET", "/foo";
        index: "GET", "/";
        many_methods: "GET", "/many_methods";
    }
}

mod head {
    run_server_tests! {
        about: "HEAD", "/about";
        favicon: "HEAD", "/favicon.ico";
        foo: "HEAD", "/foo";
        index: "HEAD", "/";
        many_methods: "HEAD", "/many_methods";
    }
}

mod post {
    run_server_tests! {
        many_methods: "POST", "/many_methods";
    }
}

mod put {
    run_server_tests! {
        many_methods: "PUT", "/many_methods";
    }
}

mod patch {
    run_server_tests! {
        many_methods: "PATCH", "/many_methods";
    }
}

mod delete {
    run_server_tests! {
        many_methods: "DELETE", "/many_methods";
    }
}

mod trace {
    run_server_tests! {
        many_methods: "TRACE", "/many_methods";
    }
}

mod options {
    run_server_tests! {
        many_methods: "OPTIONS", "/many_methods";
    }
}

mod connect {
    run_server_tests! {
        many_methods: "CONNECT", "/many_methods";
    }
}

mod z {
    run_server_tests!(SHUTDOWN_TEST_SERVER);
}
