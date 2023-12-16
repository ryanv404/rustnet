use std::env::{self, Args};
use std::process;

use rustnet::{NetResult, Router, Server, Target};

fn main() -> NetResult<()> {
    // Handle command-line options.
    let cli = Cli::parse(env::args());

    // Add some static routes and handle 404 situations.
    let router = Router::new()
        .get("/", "static/index.html")
        .get("/about", "static/about.html")
        .head("/head", "static/index.html")
        .post("/post", "static/index.html")
        .put("/put", "static/index.html")
        .patch("/patch", "static/index.html")
        .delete("/delete", "static/index.html")
        .trace("/trace", "static/index.html")
        .options("/options", "static/index.html")
        .connect("/connect", "static/index.html")
        .favicon("static/favicon.ico")
        .error_404("static/error.html");

    // Add a single path that serves different resources depending on
    // the HTTP method that is used.
    let router = router
        .route("/many_methods")
        .get(Target::Text("Hi from the GET route!"))
        .head(Target::Text("Hi from the HEAD route!"))
        .post(Target::Text("Hi from the POST route!"))
        .put(Target::Text("Hi from the PUT route!"))
        .patch(Target::Text("Hi from the PATCH route!"))
        .delete(Target::Text("Hi from the DELETE route!"))
        .trace(Target::Text("Hi from the TRACE route!"))
        .options(Target::Text("Hi from the OPTIONS route!"))
        .connect(Target::Text("Hi from the CONNECT route!"))
        .apply();

    // Start the HTTP server.
    let mut server = Server::http(&cli.addr)
        .router(router)
        .do_logging(cli.do_logging)
        .use_shutdown_route(cli.use_shutdown_route)
        .start()?;

    // Wait for the server to exit.
    if let Some(handle) = server.handle.take() {
        handle.join().unwrap();
        server.shutdown()?;
    }

    Ok(())
}

fn print_help() {
    println!(
        "\
USAGE:
    server [OPTIONS] <SERVER ADDRESS>\n
SERVER ADDRESS:
    IP:PORT      The server's IP address and port (default: 127.0.0.1:7878).\n
OPTIONS:
    --log        Enables logging of connections to the terminal.
    --shutdown   Adds a server shutdown route for testing."
    );
}

#[derive(Debug)]
struct Cli {
    do_logging: bool,
    use_shutdown_route: bool,
    addr: String,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            do_logging: false,
            use_shutdown_route: false,
            addr: "127.0.0.1:7878".to_string(),
        }
    }
}

impl Cli {
    fn new() -> Self {
        Self::default()
    }

    fn do_logging(&mut self) {
        self.do_logging = true;
    }

    fn use_shutdown_route(&mut self) {
        self.use_shutdown_route = true;
    }

    fn set_addr(&mut self, addr: &str) {
        self.addr = addr.to_string();
    }

    fn parse(args: Args) -> Self {
        let mut cli = Self::new();

        let mut args = args.skip(1);

        while let Some(opt) = args.next().as_deref() {
            if opt.is_empty() {
                return cli;
            }

            match opt {
                "--log" => cli.do_logging(),
                "--shutdown" => cli.use_shutdown_route(),
                "--help" => {
                    print_help();
                    process::exit(0);
                }
                "--" => {
                    if let Some(addr) = args.next().as_deref() {
                        cli.set_addr(addr);
                    }
                }
                unk if unk.starts_with("--") => {
                    eprintln!("Unknown option: \"{unk}\".\n");
                    print_help();
                    process::exit(1);
                }
                addr => cli.set_addr(addr),
            }
        }

        cli
    }
}
