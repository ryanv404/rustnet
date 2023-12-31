use std::collections::VecDeque;
use std::env;
use std::process;

use rustnet::{NetResult, Router, Server, ServerCli, WriteCliError};

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

    // Add some static HTML routes.
    let mut router = Router::new()
        .get("/about", "static/about.html")
        .get("/get", "static/index.html")
        .head("/head", "static/index.html")
        .post("/post", "static/index.html")
        .put("/put", "static/index.html")
        .patch("/patch", "static/index.html")
        .delete("/delete", "static/index.html")
        .trace("/trace", "static/index.html")
        .options("/options", "static/index.html")
        .connect("/connect", "static/index.html")
        // Set up a favicon and handle non-existent routes.
        .favicon("static/favicon.ico")
        .not_found("static/error.html");

    // Add a single path that serves different resources depending on
    // the HTTP method that is used.
    router = router.route("/many_methods")
        .get("Hi from the GET route!".into())
        .head("Hi from the HEAD route!".into())
        .post("Hi from the POST route!".into())
        .put("Hi from the PUT route!".into())
        .patch("Hi from the PATCH route!".into())
        .delete("Hi from the DELETE route!".into())
        .trace("Hi from the TRACE route!".into())
        .options("Hi from the OPTIONS route!".into())
        .connect("Hi from the CONNECT route!".into())
        .apply();

    // Merge the CLI router into the server router.
    router.append(&mut cli.router);

    // Build the HTTP server.
    let server = match cli.log_file.take() {
        Some(ref path) => {
            Server::http(&addr)
                .router(router)
                .do_log(cli.do_log)
                .do_debug(cli.do_debug)
                .is_test_server(cli.is_test)
                .log_file(path)
                .build()?
        },
        None => {
            Server::http(&addr)
                .router(router)
                .do_log(cli.do_log)
                .do_debug(cli.do_debug)
                .is_test_server(cli.is_test)
                .build()?
        },
    };

    if server.do_debug {
        // Debug print the `Server` and exit.
        println!("{:#?}", &server);
    } else {
        // Start the HTTP server and wait for it to exit.
        server.start()?.join()?;
    }

    Ok(())
}
