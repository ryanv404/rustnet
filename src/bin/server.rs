use std::env;

use rustnet::{NetResult, Router, Server, ServerCli};

fn main() -> NetResult<()> {
    // Handle command-line options.
    let cli = ServerCli::parse(env::args());

    // Add some static HTML routes and handle Error 404 situations.
    let router = Router::new()
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
        .favicon("static/favicon.ico")
        .not_found("static/error.html");

    // Add a single path that serves different resources depending on
    // the HTTP method that is used.
    let router = router
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

    // Start the HTTP server.
    let server = Server::new()
        .log(cli.log)
        .router(router)
        .addr(&cli.addr)
        .add_shutdown_route(cli.shutdown_route)
        .start()?;

    // Wait for the server to exit.
    server.join()?;

    Ok(())
}
