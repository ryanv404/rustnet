use std::collections::VecDeque;
use std::path::PathBuf;
use std::process;

use crate::{
    Method, Route, Router, Target, WriteCliError,
};
use crate::colors::{CLR, GRN};
use crate::config::TEST_SERVER_ADDR;

/// Contains the parsed server command line arguments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerCli {
    pub debug: bool,
    pub do_log: bool,
    pub is_test: bool,
    pub addr: Option<String>,
    pub log_file: Option<PathBuf>,
    pub router: Router,
}

impl Default for ServerCli {
    fn default() -> Self {
        Self {
            debug: false,
            do_log: false,
            is_test: false,
            addr: None,
            log_file: None,
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
                    let method = Method::from(method.as_str());

                    let target = match opt {
                        "--text" => Target::Text(target.to_string()),
                        "--file" => Target::File(PathBuf::from(target)),
                        _ => unreachable!(),
                    };

                    let route = Route::new(&method, path);
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
    IP:PORT    The server's local IP address and port.\n
{GRN}OPTIONS:{CLR}
    --debug          Prints debug information.
    --help           Prints this help message.
    --log            Enables logging of connections to stdout.
    --log-file FILE  Enables logging of connections to FILE.
    --test           Creates a test server at {TEST_SERVER_ADDR}.\n
{GRN}ROUTES:{CLR}
    --text METHOD:URI_PATH:TEXT
            Adds a route that serves text.
    --file METHOD:URI_PATH:FILE_PATH
            Adds a route that serves a file.
    --favicon FILE_PATH
            Adds a route that serves a favicon.
    --not-found FILE_PATH
            Adds a route that handles 404 Not Found responses.\n"
        );

        process::exit(0);
    }

    /// Parses command line arguments into a `ServerCli` object.
    #[must_use]
    pub fn parse_args(args: &mut VecDeque<&str>) -> Self {
        let mut cli = Self::new();

        let _ = args.pop_front();

        while let Some(opt) = args.pop_front() {
            if !opt.starts_with("--") {
                // First non-option argument is the server address.
                cli.addr = Some(opt.to_string());
                break;
            }

            match opt.len() {
                // End of options flag.
                2 if opt == "--" => {
                    // First non-option argument is the server address.
                    match args.pop_front() {
                        Some(addr) => cli.addr = Some(addr.to_string()),
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
                        cli.is_test = true;
                        cli.router.mount_shutdown_route();
                    },
                    // Add a route.
                    "--file" | "--text" => match args.pop_front() {
                        Some(arg) => cli.parse_route(opt, arg),
                        None => cli.missing_arg(opt),
                    },
                    _ => cli.unknown_opt(opt),
                },
                // Enable debugging.
                7 if opt == "--debug" => cli.debug = true,
                // Enable debugging.
                10 if opt == "--log-file" => {
                    if let Some(arg) = args.pop_front() {
                        cli.log_file = Some(PathBuf::from(arg));
                    }
                },
                // Add a favicon route.
                9 | 11 => match (opt, args.pop_front()) {
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

        cli
    }
}
