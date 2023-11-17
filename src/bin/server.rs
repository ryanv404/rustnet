use std::io;

use rustnet::Server;

fn main() -> io::Result<()> {
    // Create an HTTP server.
    let mut s = Server::http("127.0.0.1:7878");

    // Set up static routes.
    s.get("/", "static/index.html");
    s.get("/about", "static/about.html");
    s.set_favicon("static/favicon.ico");
    s.set_error_page("static/error.html");

    // Set up additional routes with other HTTP methods.
    s.post("/about");
    s.put("/about");
    s.patch("/about");
    s.delete("/about");
    s.trace("/about");
    s.options("/about");
    s.connect("localhost:7878");

    // Start the server.
    let server = s.start()?;

    // Wait for the server to finish.
    server.handle.join().unwrap();

    Ok(())
}
