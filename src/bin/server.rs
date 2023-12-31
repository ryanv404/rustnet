use std::collections::VecDeque;
use std::env;
use std::path::Path;
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
        // Set up a favicon and handle non-existent routes.
        .favicon(Path::new("static/favicon.ico"))
        .not_found(Path::new("static/error.html"));

    // Add a single path that serves different resources depending on
    // the HTTP method that is used.
    router = router.route("/many_methods")
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
