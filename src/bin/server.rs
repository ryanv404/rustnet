use std::env;

use rustnet::{NetResult, Router, Server, ServerCli};

fn main() -> NetResult<()> {
    // Handle command-line options.
    let mut cli = ServerCli::parse_args(&mut env::args());

    // Add some static HTML routes.
    let mut router = Router::new()
        .get("/", "static/index.html")
        .get("/about", "static/about.html")
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
    router = router
        .route("/many_methods")
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
    let server = Server::http(&cli.addr)
        .do_log(cli.do_log)
        .is_test_server(cli.is_test_server)
        .router(&router);

    // Print and exit if "--debug" was selected.
    if cli.debug {
        dbg!(&server);
        return Ok(());
    }

    // Start the HTTP server.
    let handle = server.start()?;

    // Wait for the server to exit.
    handle.join()?;

    Ok(())
}
