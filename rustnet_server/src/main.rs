use std::io;

use librustnet::Server;

fn main() -> io::Result<()> {
    // Create an HTTP server.
    let mut s = Server::http("127.0.0.1:7878");

    // Set up static routes.
    s.get("/", "rustnet_server/static/index.html");
    s.get("/about", "rustnet_server/static/about.html");
    s.set_favicon("rustnet_server/static/favicon.ico");
    s.set_error_page("rustnet_server/static/error.html");

    // Set up additional routes with other HTTP methods.
    s.post("/about");
    s.put("/about");
    s.patch("/about");
    s.delete("/about");
    s.trace("/about");
    s.options("/about");

    // Start the server.
    let server = s.start()?;

    // Wait for the server to finish.
    server.handle.join().unwrap();

    Ok(())
}
