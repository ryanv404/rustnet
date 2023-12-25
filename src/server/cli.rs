use std::env::Args;
use std::path::PathBuf;
use std::process;
use std::str::FromStr;

use crate::{
    Method, Route, Router, Target, WriteCliError,
};
use crate::colors::{CLR, GRN};

/// Contains the parsed server command line arguments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerCli {
    pub debug: bool,
    pub do_log: bool,
    pub is_test_server: bool,
    pub addr: String,
    pub router: Router,
}

impl Default for ServerCli {
    fn default() -> Self {
        Self {
            debug: false,
            do_log: false,
            is_test_server: false,
            addr: String::new(),
            router: Router::new()
        }
    }
}

impl WriteCliError for ServerCli {}

impl ServerCli {
    /// Returns a default `ServerCli` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses and inserts a route into the server's router.
    fn parse_route(&mut self, opt: &str, arg: &str) {
        if arg.is_empty() {
            self.missing_arg(opt);
        }

        let mut tokens = arg.splitn(3, ':');

        let tokens = (
            tokens.next(), tokens.next(), tokens.next()
        );

        match opt {
            "--favicon" => match tokens.0 {
                Some(path) => self.router.insert_favicon(path),
                None => self.invalid_arg(opt, arg),
            },
            "--not-found" => match tokens.0 {
                Some(path) => self.router.insert_not_found(path),
                None => self.invalid_arg(opt, arg),
            },
            "--text" | "--file" => match tokens {
                (Some(method), Some(path), Some(target)) => {
                    let method = method.to_ascii_uppercase();
                    let Ok(method) = Method::from_str(method.as_str()) else {
                        return self.invalid_arg(opt, arg);
                    };

                    let route = Route::new(&method, path);

                    let target = match opt {
                        "--text" => Target::Text(target.to_string()),
                        "--file" => Target::File(PathBuf::from(target)),
                        _ => unreachable!(),
                    };

                    self.router.mount(route, target);
                },
                (_, _, _) => self.invalid_arg(opt, arg),
            },
            _ => self.unknown_opt(opt),
        }
    }

    /// Prints the server help message and exists the program.
    pub fn print_help(&self) {
        eprintln!(
            "\
{GRN}USAGE:{CLR}
    http_server [OPTIONS] [--] <SERVER ADDRESS>\n
{GRN}SERVER ADDRESS:{CLR}
    IP:PORT    The server's IP address and port.\n
{GRN}OPTIONS:{CLR}
    --debug      Prints debug information.
    --help       Prints this help message.
    --log        Enables logging of connections to stdout.
    --test       Creates a test server with a shutdown route.\n
{GRN}ROUTES:{CLR}
    --text METHOD:URI_PATH:TEXT
            Adds a route that serves text.
    --file METHOD:URI_PATH:FILE_PATH
            Adds a route that serves a file.
    --favicon FILE_PATH
            Adds a route that serves a favicon icon.
    --not-found FILE_PATH
            Adds a route that handles 404 Not Found responses.\n"
        );

        process::exit(0);
    }

    /// Parses command line arguments into a `ServerCli` object.
    #[must_use]
    pub fn parse_args(args: &mut Args) -> Self {
        let mut cli = Self::new();

        let _ = args.next();

        while let Some(opt) = args.next().as_deref() {
            if !opt.starts_with("--") {
                // First non-option argument is the server address.
                cli.addr = opt.to_string();
                return cli;
            }

            match opt.len() {
                // End of options flag.
                2 if opt == "--" => {
                    // First non-option argument is the server address.
                    match args.next() {
                        Some(addr) => cli.addr = addr,
                        None => cli.missing_arg("SERVER ADDRESS"),
                    }

                    break;
                },
                // Enable logging of new connections.
                5 if opt == "--log" => cli.do_log = true,
                6 => match opt {
                    // Print help message.
                    "--help" => cli.print_help(),
                    // Make the server a test server.
                    "--test" => {
                        cli.is_test_server = true;
                        cli.router.mount_shutdown_route();
                    },
                    // Add a route.
                    "--file" | "--text" => match args.next().as_deref() {
                        Some(arg) => cli.parse_route(opt, arg),
                        None => cli.missing_arg(opt),
                    },
                    _ => cli.unknown_opt(opt),
                },
                // Enable debugging.
                7 if opt == "--debug" => cli.debug = true,
                // Add a favicon route.
                9 | 11 => match (opt, args.next().as_deref()) {
                    ("--favicon" | "--not-found", Some(arg)) => {
                        cli.parse_route(opt, arg);
                    },
                    ("--favicon" | "--not-found", None) => {
                        cli.missing_arg(opt);
                    },
                    (_, _) => cli.unknown_opt(opt),
                },
                // Unknown option.
                _ => cli.unknown_opt(opt),
            }
        }

        if cli.addr.is_empty() {
            cli.missing_arg("SERVER ADDRESS");
        }

        cli
    }
}
