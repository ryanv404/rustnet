use std::env;
use std::thread;
use std::time::Duration;

use librustnet::{NetResult, Router, Server};

fn main() -> NetResult<()> {
    // Handle command-line options.
    let args: Vec<String> = env::args().collect();

    let (should_log, use_shutdown_route) = match args.get(1) {
        None => (false, false),
        Some(opt_a) if opt_a == "--enable-logging" => args.get(2).map_or((true, false),
            |opt_b| {
                if opt_b == "--use-shutdown-route" {
                    (true, true)
                } else {
                    eprintln!("Unknown option: \"{opt_b}\".\n\n\
                        To enable logging, use \"--enable-logging\".\n\
                        To add a shutdown route for testing, use \"--use-shutdown-route\".");
                    (false, false)
                }
        }),
        Some(opt_a) if opt_a == "--use-shutdown-route" => args.get(2).map_or((false, true),
            |opt_b| {
                if opt_b == "--enable-logging" {
                    (true, true)
                } else {
                    eprintln!("Unknown option: \"{opt_b}\".\n\n\
                        To enable logging, use \"--enable-logging\".\n\
                        To add a shutdown route for testing, use \"--use-shutdown-route\".");
                    (false, false)
                }
        }),
        Some(unk) => {
            eprintln!("Unknown option: \"{unk}\".\n\n\
                To enable logging, use \"--enable-logging\".\n\
                To add a shutdown route for testing, use \"--use-shutdown-route\".");
            (false, false)
        },
    };

    // Set up the router.
    let router = Router::new()
        // Add routes that serve static resources.
        .get("/", "static/index.html")
        .get("/about", "static/about.html")
        .favicon("static/favicon.ico")
        .error_404("static/error.html");

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

    // Create and start the HTTP server.
    let mut server = Server::http("127.0.0.1:7878")
        .router(router)
        .log(should_log)
        .shutdown_route(use_shutdown_route)
        .start()?;

    // Wait for the server's listener thread to exit.
    if let Some(handle) = server.handle.take() {
        while !handle.is_finished() {
            thread::sleep(Duration::from_millis(200));
        }

        handle.join().unwrap();
        server.shutdown()?;
    }

    Ok(())
}
