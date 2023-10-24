use std::io;

use rustnet::Server;

fn main() -> io::Result<()> {
    // Create an HTTP server.
    let mut s = Server::new("127.0.0.1:7878");

    // Set up endpoints.
    s.get("/", "examples/static/index.html");
    s.get("/about", "examples/static/about.html");

    // Set the location of the favicon.
    s.set_favicon("examples/static/favicon.ico");

    // Set the default 404 Not Found page for all other endpoints.
    s.set_error_page("examples/static/error.html");

    // Start the server.
    s.start()?;

    Ok(())
}
