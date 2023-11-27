use std::io;

use librustnet::{Body, Server};

fn main() -> io::Result<()> {
    // Create an HTTP server.
    let mut builder = Server::builder()
        // Set the address on which the server will listen.
        .addr("127.0.0.1:7878")

        // Set up static routes.
        .get("/", "server/static/index.html")
        .get("/about", "server/static/about.html")
        .favicon("server/static/favicon.ico")
        .error_page("server/static/error.html")

        // Set up additional routes with other HTTP methods.
        .post("/about")
        .put("/about")
        .patch("/about")
        .delete("/about")
        .trace("/about")
        .options("/about")
        .connect("127.0.0.1:1234")

        // Set up routes using handler functions.
        .get_with_handler("/handler", |req, res| {
            let msg = format!("REQUEST:\n{}\n\nRESPONSE:\n{}\n", req, res);
            res.body = Body::Text(msg);
        });

    // Enable logging to stdout.
    builder.logging(true);

    // Start the server.
    let server = builder.start()?;

    // Wait for the server to finish.
    server.thread.join().unwrap();

    Ok(())
}
