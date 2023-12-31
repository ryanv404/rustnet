use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;

use crate::{Method, Route, Router, WriteCliError, TEST_SERVER_ADDR};
use crate::style::colors::{BR_GRN, CLR};

/// Contains the parsed server command line arguments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerCli {
    pub do_log: bool,
    pub do_debug: bool,
    pub is_test: bool,
    pub router: Router,
    pub addr: Option<String>,
    pub log_file: Option<PathBuf>,
}

impl Default for ServerCli {
    fn default() -> Self {
        Self {
            do_log: false,
            do_debug: false,
            is_test: false,
            router: Router::new(),
            addr: None,
            log_file: None
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
    pub fn parse_route(&mut self, opt: &str, arg: &str) {
        if arg.is_empty() {
            self.missing_arg(opt);
        }

        let mut tokens = arg.splitn(3, ':');

        match opt {
            "-I" | "--favicon" => match tokens.next() {
                Some(path) => {
                    let route = Route::Get("/favicon.ico".into());
                    let target = Path::new(path).to_path_buf();
                    self.router.mount(route, target.into());
                },
                None => self.invalid_arg(opt, arg),
            },
            "-0" | "--not-found" => match tokens.next() {
                Some(path) => {
                    let route = Route::NotFound;
                    let target = Path::new(path).to_path_buf();
                    self.router.mount(route, target.into());
                },
                None => self.invalid_arg(opt, arg),
            },
            "-T" | "--text" | "-F" | "--file" => {
                let (Some(method), Some(path), Some(target)) = (
                    tokens.next(),
                    tokens.next(),
                    tokens.next()
                ) else {
                    self.invalid_arg(opt, arg);
                    return;
                };

                let method = method.to_ascii_uppercase();
                let Ok(method) = Method::from_str(method.as_str()) else {
                    return self.invalid_arg(opt, arg);
                };

                let route = Route::new(method, path.to_ascii_lowercase());

                match opt {
                    "-T" | "--text" => {
                        let target = String::from(target);
                        self.router.mount(route, target.into());
                    },
                    "-F" | "--file" => {
                        let target = Path::new(target).to_path_buf();
                        self.router.mount(route, target.into());
                    },
                    _ => unreachable!(),
                }
            },
            _ => unreachable!(),
        }
    }

    /// Prints the server help message and exists the program.
    pub fn print_help(&self) {
        eprintln!(
            "\
{BR_GRN}USAGE:{CLR}
    http_server [OPTIONS] [--] <SERVER ADDRESS>\n
{BR_GRN}SERVER ADDRESS:{CLR}
    IP:PORT    The server's local IP address and port.\n
{BR_GRN}OPTIONS:{CLR}
    -d, --debug          Prints debug information.
    -h, --help           Prints this help message.
    -l, --log            Enables logging of connections to stdout.
    -f, --log-file FILE  Enables logging of connections to FILE.
    -t, --test           Creates a test server at {TEST_SERVER_ADDR}.\n
{BR_GRN}ROUTES:{CLR}
    -T, --text METHOD:URI_PATH:TEXT
        Adds a route that serves text.
    -F, --file METHOD:URI_PATH:FILE_PATH
        Adds a route that serves a file.
    -I, --favicon FILE_PATH
        Adds a route that serves a favicon.
    -0, --not-found FILE_PATH
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
            match opt {
                // First argument after "--" is the server address.
                "--" => match args.pop_front() {
                    Some(arg) => {
                        cli.addr = Some(arg.to_string());
                        break;
                    },
                    None => cli.missing_arg("SERVER ADDRESS"),
                },
                // Handle options.
                _ if opt.starts_with('-') => match opt {
                    // Enable logging of new connections.
                    "-l" | "--log" => cli.do_log = true,
                    // Enable debug printing.
                    "-d" | "--debug" => cli.do_debug = true,
                    // Print help message.
                    "-h" | "--help" => cli.print_help(),
                    // Make the server a test server.
                    "-t" | "--test" => {
                        cli.is_test = true;
                        cli.router.mount_shutdown_route();
                    },
                    // Set a local log file.
                    "-f" | "--log-file" => match args.pop_front() {
                        Some(arg) => {
                            cli.do_log = true;
                            cli.log_file = Some(PathBuf::from(arg));
                        },
                        None => cli.missing_arg(opt),
                    },
                    // Add a route.
                    "-F" | "--file"
                        | "-I" | "--favicon"
                        | "-0" | "--not-found"
                        | "-T" | "--text" => match args.pop_front() {
                        Some(arg) => cli.parse_route(opt, arg),
                        None => cli.missing_arg(opt),
                    },
                    // Unknown option.
                    _ => cli.unknown_opt(opt),
                },
                // First non-option argument is the server address.
                _ => {
                    cli.addr = Some(opt.to_string());
                    break;
                },
            }
        }

        cli
    }
}
