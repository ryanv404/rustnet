use std::env;
use std::io;

use librustnet::{Router, Server};

fn main() -> io::Result<()> {
    // Set up the router.
    let router = Router::new()
        // Add routes that serve static resources.
        .get("/", "server/static/index.html")
        .get("/about", "server/static/about.html")
        .favicon("server/static/favicon.ico")
        .error_404("server/static/error.html");

    // Add a single URI path that responds differently to each HTTP method.
    let router = router
        .route("/many_methods")
            .get(|res| res.body.text("Hi from the GET route!"))
            .post(|res| res.body.text("Hi from the POST route!"))
            .put(|res| res.body.text("Hi from the PUT route!"))
            .patch(|res| res.body.text("Hi from the PATCH route!"))
            .delete(|res| res.body.text("Hi from the DELETE route!"))
            .trace(|res| res.body.text("Hi from the TRACE route!"))
            .options(|res| res.body.text("Hi from the OPTIONS route!"))
            .connect(|res| res.body.text("Hi from the CONNECT route!"))
            .apply();

    // Check if logging has been enabled.
    let should_log = match env::args().nth(1) {
        None => false,
        Some(opt) if opt == "--enable-logging" => true,
        Some(unk) => {
            println!("\
                Unknown option: \"{unk}\".\n\n\
                To enable logging, use \"--enable-logging\".\n");
            return Ok(());
        },
    };

    // Create and run the HTTP server.
    Server::http("127.0.0.1:7878")
        .router(router)
        .log(should_log)
        .start()?;

    Ok(())
}
