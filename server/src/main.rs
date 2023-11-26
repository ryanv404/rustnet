use std::io;

use librustnet::Server;

fn main() -> io::Result<()> {
    // Create an HTTP server.
    let mut server = Server::builder()
        // Set the address on which the server will listen.
        .addr("127.0.0.1:7878")

        // Set up static routes.
        .get("/", "server/static/index.html")
        .get("/about", "server/static/about.html")
        .set_favicon("server/static/favicon.ico")
        .set_error_page("server/static/error.html")

        // Set up additional routes with other HTTP methods.
        .post("/about")
        .put("/about")
        .patch("/about")
        .delete("/about")
        .trace("/about")
        .options("/about")
        .connect("127.0.0.1:1234")

        // Set up a route using a route handler function.
        .get_with_handler("/handler", |req, res| {
            let msg = format!("REQUEST:\n{}\n\nRESPONSE:\n{}\n", req, res);
            res.body = Some(msg.into_bytes());
        });

    // Enable logging to stdout.
    server.enable_logging();

    // Start the server.
    let handle = server.build()?.start()?;

    // Wait for the server to finish.
    handle.thread.join().unwrap();

    Ok(())
}
