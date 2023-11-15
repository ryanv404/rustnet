use std::io;

use rustnet::Server;

fn main() -> io::Result<()> {
    // Create an HTTP server.
    let mut s = Server::http("127.0.0.1:7878");

    // Set up routes.
    s.get("/", "static/index.html");
    s.get("/about", "static/about.html");

    // Set the location of the favicon.
    s.set_favicon("static/favicon.ico");

    // Set the default 404 Not Found page for unmatched routes.
    s.set_error_page("static/error.html");

    // Start the server.
    let server = s.start()?;

    // Wait for the server to finish.
    server.handle.join().unwrap();

    Ok(())
}
