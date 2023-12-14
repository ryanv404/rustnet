#[macro_use]
mod common;

mod get {
    run_server_test! {
        about: "GET", "/about";
        foo: "GET", "/foo";
        index: "GET", "/";
        many_methods: "GET", "/many_methods";
    }
}

mod head {
    run_server_test! {
        about: "HEAD", "/about";
        favicon: "HEAD", "/favicon.ico";
        foo: "HEAD", "/foo";
        index: "HEAD", "/";
        many_methods: "HEAD", "/many_methods";
    }
}

mod post {
    run_server_test! {
        many_methods: "POST", "/many_methods";
    }
}

mod put {
    run_server_test! {
        many_methods: "PUT", "/many_methods";
    }
}

mod patch {
    run_server_test! {
        many_methods: "PATCH", "/many_methods";
    }
}

mod delete {
    run_server_test! {
        many_methods: "DELETE", "/many_methods";
    }
}

mod trace {
    run_server_test! {
        many_methods: "TRACE", "/many_methods";
    }
}

mod options {
    run_server_test! {
        many_methods: "OPTIONS", "/many_methods";
    }
}

mod connect {
    run_server_test! {
        many_methods: "CONNECT", "/many_methods";
    }
}
