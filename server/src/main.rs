use std::io;

use librustnet::{Router, Server};

fn main() -> io::Result<()> {
    // Set up the router.
    let router = Router::new()
        // Set up routes using static resources.
        .get("/", "server/static/index.html")
        .get("/about", "server/static/about.html")
        .favicon("server/static/favicon.ico")
        .error_404("server/static/error.html")

        // Set up additional routes using other HTTP methods.
        .route("/many_methods")
            .get(|_req, res| res.body.text("Hi from the GET route!"))
            .post(|_req, res| res.body.text("Hi from the POST route!"))
            .put(|_req, res| res.body.text("Hi from the PUT route!"))
            .patch(|_req, res| res.body.text("Hi from the PATCH route!"))
            .delete(|_req, res| res.body.text("Hi from the DELETE route!"))
            .trace(|_req, res| res.body.text("Hi from the TRACE route!"))
            .options(|_req, res| res.body.text("Hi from the OPTIONS route!"))
            .connect(|_req, res| res.body.text("Hi from the CONNECT route!"))
            .apply();

    // Create an HTTP server.
    Server::builder()
        .addr("127.0.0.1:7878")
        .router(router)
        .start()?;

    Ok(())
}
