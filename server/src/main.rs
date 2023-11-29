use std::{env, io};

use librustnet::{Router, Server};

fn main() -> io::Result<()> {
    // Enable logging from the command-line (default is no logging).
    let do_logging = match env::args().nth(1) {
        Some(opt) if opt.eq_ignore_ascii_case("--enable-logging") => true,
        Some(unk) => {
            println!("\
                Unknown option: \"{unk}\".\n\n\
                To enable logging, use \"--enable-logging\".\n");
            return Ok(());
        },
        None => false,
    };

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

    // Create and run the HTTP server.
    Server::http("127.0.0.1:7878")
        .router(router)
        .log(do_logging)
        .start()?;

    Ok(())
}
