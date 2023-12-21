use std::env;

use rustnet::{NetResult, Router, Server, ServerCli};

fn main() -> NetResult<()> {
    // Handle command-line options.
    let cli = ServerCli::parse_args(env::args());

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
        // Set up favicon and handle non-existent routes.
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

    // Apply CLI routes to server.
    for (name, value) in cli.router.0.iter() {
        router.0.insert(name.clone(), value.clone());
    }

    // Start the HTTP server.
    let server = if cli.is_test {
        Server::test(&cli.addr, router)
            .log_connections(cli.do_logging)
            .start()?
    } else {
        Server::http(&cli.addr)
            .router(router)
            .log_connections(cli.do_logging)
            .start()?
    };

    // Wait for the server to exit.
    server.join()?;

    Ok(())
}
