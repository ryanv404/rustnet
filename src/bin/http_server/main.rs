use std::collections::VecDeque;
use std::env;
use std::path::Path;
use std::process;

use rustnet::{NetResult, Router, Server, WriteCliError};

mod cli;

use cli::ServerCli;

fn main() -> NetResult<()> {
    let args = env::args().collect::<VecDeque<String>>();

    let mut args = args
        .iter()
        .map(|s| s.as_ref())
        .collect::<VecDeque<&str>>();

    let mut cli = ServerCli::parse_args(&mut args);

    let Some(addr) = cli.addr.take() else {
        cli.missing_arg("SERVER ADDRESS");
        process::exit(1);
    };

    let mut router = Router::new();

    // Add some static resource routes.
    let _ = router
        .get("/about", Path::new("static/about.html"))
        .get("/get", Path::new("static/index.html"))
        .head("/head", Path::new("static/index.html"))
        .post("/post", Path::new("static/index.html"))
        .put("/put", Path::new("static/index.html"))
        .patch("/patch", Path::new("static/index.html"))
        .delete("/delete", Path::new("static/index.html"))
        .trace("/trace", Path::new("static/index.html"))
        .options("/options", Path::new("static/index.html"))
        .connect("/connect", Path::new("static/index.html"))
        .favicon(Path::new("static/favicon.ico"))
        .not_found(Path::new("static/error.html"));

    // Add a single path that serves different resources depending on
    // the HTTP method that is used.
    let mut router = router.route("/many_methods")
        .get("Hi from the GET route!")
        .head("Hi from the HEAD route!")
        .post("Hi from the POST route!")
        .put("Hi from the PUT route!")
        .patch("Hi from the PATCH route!")
        .delete("Hi from the DELETE route!")
        .trace("Hi from the TRACE route!")
        .options("Hi from the OPTIONS route!")
        .connect("Hi from the CONNECT route!")
        .apply();

    // Merge the CLI router into the server router.
    router.append(&mut cli.router);

    let mut builder = Server::http(&addr);

    let _ = builder
        .router(&mut router)
        .do_log(cli.do_log)
        .do_debug(cli.do_debug)
        .is_test_server(cli.is_test);

    // Build the HTTP server.
    let server = match cli.log_file.take() {
        Some(ref path) => builder.log_file(path).build()?,
        None => builder.build()?,
    };

    if server.do_debug {
        // Debug print the `Server` and exit.
        println!("{:#?}", &server);
        return Ok(());
    }

    // Start the HTTP server and wait for it to exit.
    server.start()?.join()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use rustnet::{Method, Route, Target};

    #[test]
    fn parse_args() {
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
