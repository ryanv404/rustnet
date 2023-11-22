use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const BLU: &str = "\x1b[94m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

fn main() {
    let name = env!("CARGO_BIN_NAME");
    let help_msg = format!("\
        {PURP}USAGE:{CLR} {name} <TEST_GROUP>\n\n\
        {PURP}TEST_GROUP OPTIONS:{CLR}\n    \
            client    Run only client tests.\n    \
            server    Run only server tests.\n    \
            all       Run all tests.\n");

    env::args().nth(1).map_or_else(
        || println!("{help_msg}"),
        |arg| {
            let mut results = TestResults::default();

            match arg.as_str() {
                "client" => {
                    clean_up(true);
                    run_client_tests(&mut results);
                    print_final_results(&results);
                    clean_up(false);
                },
                "server" => {
                    clean_up(true);
                    run_server_tests(&mut results);
                    print_final_results(&results);
                    clean_up(false);
                },
                "all" => {
                    clean_up(true);
                    run_server_tests(&mut results);
                    run_client_tests(&mut results);
                    print_final_results(&results);
                    clean_up(false);
                },
                _ => {
                    println!("{RED}Unknown argument: `{arg}`{CLR}\n");
                    println!("{help_msg}");
                },
            }
        });
}

macro_rules! test_client {
    ($label:ident: $uri_path:literal, $tracker:expr) => {
        fn $label(tracker: &mut TestResults) {
            tracker.client_total += 1;

            let file = concat!(stringify!($label), ".txt");
            let filepath = format!("examples/client_tests/{file}");

            let client_bin = if cfg!(windows) {
                "target/debug/examples/client.exe"
            } else {
                "target/debug/examples/client"
            };

            // httpbin.org:80
            let uri = concat!("54.86.118.241:80", $uri_path);

            let output = Command::new(client_bin)
                .args(["--testing", uri])
                .output()
                .unwrap();

            if output_matches_expected(&output.stdout, &filepath) {
                tracker.client_passed += 1;
                println!("[{GRN}✔{CLR}] GET {}", $uri_path);
            } else {
                println!("[{RED}✗{CLR}] GET {}", $uri_path);
            }
        }

        $label($tracker)
    };
}

macro_rules! test_server {
    ($label:ident: $method:literal, $uri_path:literal, $tracker:expr) => {
        fn $label(tracker: &mut TestResults) {
            tracker.server_total += 1;

            let uri = concat!("127.0.0.1:7878", $uri_path);
            let file = concat!(stringify!($label), ".txt");
            let filepath = format!("examples/server_tests/{file}");

            let output = match $method {
                "HEAD" => Command::new("curl")
                    .args(["--silent", "--include", "--head", uri])
                    .output()
                    .unwrap(),
                _ => Command::new("curl")
                    .args(["--silent", "--include", "-X", $method, uri])
                    .output()
                    .unwrap(),
            };

            if output_matches_expected(&output.stdout, &filepath) {
                tracker.server_passed += 1;
                println!("[{GRN}✔{CLR}] {} {}", $method, $uri_path);
            } else {
                println!("[{RED}✗{CLR}] {} {}", $method, $uri_path);
            }
        }

        $label($tracker)
    };
}

#[derive(Default)]
struct TestResults {
    client_passed: u16,
    client_total: u16,
    server_passed: u16,
    server_total: u16,
}

fn print_final_results(results: &TestResults) {
    if results.client_total > 0 || results.server_total > 0 {
        println!("{BLU}+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+{CLR}");
    }

    if results.client_total > 0 {
        if results.client_passed == results.client_total {
            println!(
                "CLIENT: {GRN}{} out of {} tests passed.{CLR}",
                results.client_passed,
                results.client_total
            );
        } else {
            println!(
                "CLIENT: {RED}{} out of {} tests failed.{CLR}",
                results.client_total - results.client_passed,
                results.client_total
            );
        }
    }

    if results.server_total > 0 {
        if results.server_passed == results.server_total {
            println!(
                "SERVER: {GRN}{} out of {} tests passed.{CLR}",
                results.server_passed,
                results.server_total
            );
        } else {
            println!(
                "SERVER: {RED}{} out of {} tests failed.{CLR}",
                results.server_total - results.server_passed,
                results.server_total
            );
        }
    }

    if results.client_total > 0 || results.server_total > 0 {
        println!("{BLU}+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+{CLR}");
    }
}

fn run_client_tests(results: &mut TestResults) {
    if build_client().is_some() {
        println!("Client tests:");
        test_client!(get_json: "/json", results);
        test_client!(get_xml: "/xml", results);
        test_client!(get_png: "/image/png", results);
        test_client!(get_svg: "/image/svg", results);
        test_client!(get_webp: "/image/webp", results);
        test_client!(get_text: "/robots.txt", results);
        test_client!(get_utf8: "/encoding/utf8", results);
        test_client!(get_html: "/html", results);
        test_client!(get_deny: "/deny", results);
        println!();
    }
}

fn run_server_tests(results: &mut TestResults) {
    if let Some(server) = build_and_start_server().as_mut() {
        println!("Server tests:");
        test_server!(get_about: "GET", "/about", results);
        test_server!(get_foo: "GET", "/foo", results);
        test_server!(get_index: "GET", "/", results);
        test_server!(head_index: "HEAD", "/", results);
        test_server!(head_about: "HEAD", "/about", results);
        test_server!(head_foo: "HEAD", "/foo", results);
        test_server!(head_favicon: "HEAD", "/favicon.ico", results);
        test_server!(post_about: "POST", "/about", results);
        test_server!(put_about: "PUT", "/about", results);
        test_server!(patch_about: "PATCH", "/about", results);
        test_server!(delete_about: "DELETE", "/about", results);
        test_server!(trace_about: "TRACE", "/about", results);
        test_server!(options_about: "OPTIONS", "/about", results);
        println!();
        server.kill().unwrap();
    }
}

fn build_client() -> Option<()> {
    print!("Building client...");
    io::stdout().flush().unwrap();

    let build_status = Command::new("cargo")
        .args(["build", "--example", "client"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    if build_status.success() {
        println!("{GRN}✔{CLR}\n");
        Some(())
    } else {
        println!("{RED}✗{CLR}");
        None
    }
}

fn build_and_start_server() -> Option<Child> {
    print!("Building server...");
    io::stdout().flush().unwrap();

    let build_status = Command::new("cargo")
        .args(["build", "--example", "server"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    if build_status.success() {
        println!("{GRN}✔{CLR}");
    } else {
        println!("{RED}✗{CLR}");
        return None;
    }

    print!("Starting server...");
    io::stdout().flush().unwrap();

    let server_bin = if cfg!(windows) {
        "target/debug/examples/server.exe"
    } else {
        "target/debug/examples/server"
    };

    let mut server = Command::new(server_bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    thread::sleep(Duration::from_millis(100));

    let max_attempts = 5;
    let mut attempt_num = 0;

    loop {
        attempt_num += 1;
        thread::sleep(Duration::from_millis(500));

        let conn_status = Command::new("curl")
            .arg("127.0.0.1:7878/")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();

        if conn_status.success() {
            println!("{GRN}✔{CLR}\n");
            return Some(server);
        } else if attempt_num > max_attempts {
            println!("{RED}✗{CLR}\n\nServer took too long to go live.");
            server.kill().unwrap();
            return None;
        }
    }
}

fn output_matches_expected(
    test_output: &[u8],
    expected_output_file: &str,
) -> bool {
    let output = String::from_utf8_lossy(test_output);

    let mut expected = String::new();
    let mut f = File::open(expected_output_file).unwrap();
    f.read_to_string(&mut expected).unwrap();

    let output: Vec<&str> = output
        .split('\n')
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                Some(trimmed)
            } else {
                None
            }
        })
        .collect();

    let expected: Vec<&str> = expected
        .split('\n')
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                Some(trimmed)
            } else {
                None
            }
        })
        .collect();

    let output = output.join("\n");
    let expected = expected.join("\n");

    output == expected
}

fn clean_up(do_log: bool) {
    if do_log {
        print!("Cleaning old build artifacts...");
        io::stdout().flush().unwrap();
    }

    let clean_status = Command::new("cargo")
        .arg("clean")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    if do_log && clean_status.success() {
        println!("{GRN}✔{CLR}");
    } else if do_log {
        println!("{RED}✗{CLR}");
    }
}
