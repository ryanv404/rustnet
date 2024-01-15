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

#[cfg(test)]
mod parse {
    use std::collections::{BTreeSet, VecDeque};
    use std::path::{Path, PathBuf};
    use rustnet::{Method, Route, Router, ServerCli, Target};

    #[test]
    fn cli_args() {
        let mut args = VecDeque::from([
            "./server", "--test", "-d",
            "--log-file", "./log_file.txt",
            "-I", "./favicon.ico",
            "-0", "./error_404.html",
            "-T", "pUt:/put:test message1.",
            "-T", "pAtch:/patCh:test message2.",
            "-T", "DeleTe:/dEletE:test message3.",
            "-F", "GeT:/geT:./static/get.html",
            "-F", "HEaD:/hEad:./static/head.html",
            "-F", "pOst:/poSt:./static/post.html",
            "127.0.0.1:7879"
        ]);

        let test_cli = ServerCli::parse_args(&mut args);

        let router = Router(BTreeSet::from([
            Route {
                method: Method::Shutdown,
                path: None,
                target: Target::Shutdown
            },
            Route {
                method: Method::Get,
                path: Some("/favicon.ico".into()),
                target: Target::Favicon(Path::new("./favicon.ico").into())
            },
            Route {
                method: Method::Any,
                path: None,
                target: Path::new("./error_404.html").into()
            },
            Route {
                method: Method::Get,
                path: Some("/get".into()),
                target: Path::new("./static/get.html").into()
            },
            Route {
                method: Method::Post,
                path: Some("/post".into()),
                target: Path::new("./static/post.html").into()
            },
            Route {
                method: Method::Head,
                path: Some("/head".into()),
                target: Path::new("./static/head.html").into()
            },
            Route {
                method: Method::Put,
                path: Some("/put".into()),
                target: "test message1.".into()
            },
            Route {
                method: Method::Patch,
                path: Some("/patch".into()),
                target: "test message2.".into()
            },
            Route {
                method: Method::Delete,
                path: Some("/delete".into()),
                target: "test message3.".into()
            }
        ]));

        let expected_cli = ServerCli {
            do_log: true,
            do_debug: true,
            is_test: true,
            addr: Some("127.0.0.1:7879".to_string()),
            log_file: Some(PathBuf::from("./log_file.txt")),
            router
        };

        assert_eq!(test_cli, expected_cli);
    }
}

// Use alphabetical module naming to shut down the test server at the end.
#[cfg(test)]
mod z {
    shutdown_test_server!();
}
