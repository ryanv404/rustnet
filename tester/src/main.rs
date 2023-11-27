use std::borrow::Borrow;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use librustnet::StatusLine;

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

#[cfg(windows)]
const SERVER_FILE: &str = "server.exe";
#[cfg(not(windows))]
const SERVER_FILE: &str = "server";

#[cfg(windows)]
const CLIENT_FILE: &str = "client.exe";
#[cfg(not(windows))]
const CLIENT_FILE: &str = "client";

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const BLU: &str = "\x1b[94m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

#[derive(Default)]
struct TestResults {
    client_passed: u16,
    client_total: u16,
    server_passed: u16,
    server_total: u16,
    status_passed: u16,
    status_total: u16,
}

fn main() {
    let name = env!("CARGO_BIN_NAME");
    let help_msg = format!("\
        {PURP}Usage:{CLR} {name} <TEST-GROUP>\n\n\
        {PURP}TEST-GROUP Options:{CLR}\n    \
            client    Run only client tests.\n    \
            server    Run only server tests.\n    \
            all       Run all tests.\n");

    env::args().nth(1).map_or_else(
        || println!("{help_msg}"),
        |arg| {
            let mut results = TestResults::default();

            match arg.as_str() {
                "client" => {
                    remove_old_builds("client");
                    if build_client().is_some() {
                        run_client_tests(&mut results);
                        print_final_results(&results);
                    }
                },
                "server" => {
                    remove_old_builds("server");
                    if build_client().is_some() {
                        run_server_tests(&mut results);
                        print_final_results(&results);
                    }
                },
                "all" => {
                    remove_old_builds("all");
                    if build_client().is_some() {
                        run_client_tests(&mut results);
                        run_server_tests(&mut results);
                        print_final_results(&results);
                    }
                },
                _ => {
                    println!("{RED}Unknown argument: `{arg}`{CLR}\n");
                    println!("{help_msg}");
                },
            }
        });
}

macro_rules! test_client {
    ($label:ident: $method:literal, $uri_path:literal, $tracker:expr) => {
        fn $label(tracker: &mut TestResults) {
            tracker.client_total += 1;

            let exp_file: PathBuf = [
                CRATE_ROOT,
                "..",
                "client",
                "test_data",
                concat!(stringify!($label), ".txt")
            ].iter().collect();

            let client_bin: PathBuf = [
                CRATE_ROOT,
                "..",
                "target",
                "debug",
                CLIENT_FILE
            ].iter().collect();

            // httpbin.org:80
            let uri = concat!("54.86.118.241:80", $uri_path);

            let output = Command::new(&client_bin)
                .args([
                    "--method",
                    $method,
                    "--client-tests",
                    "--",
                    uri
                ])
                .output()
                .unwrap();

            match get_result(&output.stdout, &exp_file) {
                Some((ref out, ref exp)) => {
                    println!("[{RED}✗{CLR}] {} {}", $method, $uri_path);
                    println!("OUTPUT:\n{out}\n\nEXPECTED:\n{exp}\n");
                },
                None => {
                    tracker.client_passed += 1;
                    println!("[{GRN}✔{CLR}] {} {}", $method, $uri_path);
                },
            }
        }

        $label($tracker)
    };
}

macro_rules! test_server {
    ($label:ident: $method:literal, $uri_path:literal, $tracker:expr) => {
        fn $label(tracker: &mut TestResults) {
            tracker.server_total += 1;

            let exp_file: PathBuf = [
                CRATE_ROOT,
                "..",
                "server",
                "test_data",
                concat!(stringify!($label), ".txt")
            ].iter().collect();

            let client_bin: PathBuf = [
                CRATE_ROOT,
                "..",
                "target",
                "debug",
                CLIENT_FILE
            ].iter().collect();

            let output = if $method == "CONNECT" {
                Command::new(&client_bin)
                    .args([
                        "--server-tests",
                        "--method",
                        $method,
                        "--path",
                        $uri_path,
                        "--",
                        "127.0.0.1:7878"
                    ])
                    .output()
                    .unwrap()
            } else {
                let uri = concat!("127.0.0.1:7878", $uri_path);
                Command::new(&client_bin)
                    .args([
                        "--server-tests",
                        "--method",
                        $method,
                        "--",
                        uri
                    ])
                    .output()
                    .unwrap()
            };

            match get_result(&output.stdout, &exp_file) {
                Some((ref out, ref exp)) => {
                    println!("[{RED}✗{CLR}] {} {}", $method, $uri_path);
                    println!("OUTPUT:\n{out}\n\nEXPECTED:\n{exp}\n");
                },
                None => {
                    tracker.server_passed += 1;
                    println!("[{GRN}✔{CLR}] {} {}", $method, $uri_path);
                },
            }
        }

        $label($tracker)
    };
}

fn print_final_results(results: &TestResults) {
    if results.client_total > 0 ||
        results.server_total > 0 ||
        results.status_total > 0
    {
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

    if results.status_total > 0 {
        if results.status_passed == results.status_total {
            println!(
                "STATUS: {GRN}{} out of {} tests passed.{CLR}",
                results.status_passed,
                results.status_total
            );
        } else {
            println!(
                "STATUS: {RED}{} out of {} tests failed.{CLR}",
                results.status_total - results.status_passed,
                results.status_total
            );
        }
    }

    if results.client_total > 0 ||
        results.server_total > 0 ||
        results.status_total > 0
    {
        println!("{BLU}+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+{CLR}");
    }
}

fn run_client_tests(results: &mut TestResults) {
    println!("\n~~~~~~~~~~~~\nClient Tests\n~~~~~~~~~~~~");
    test_client!(get_json: "GET", "/json", results);
    test_client!(get_xml: "GET", "/xml", results);
    test_client!(get_png: "GET", "/image/png", results);
    test_client!(get_svg: "GET", "/image/svg", results);
    test_client!(get_webp: "GET", "/image/webp", results);
    test_client!(get_text: "GET", "/robots.txt", results);
    test_client!(get_utf8: "GET", "/encoding/utf8", results);
    test_client!(get_html: "GET", "/html", results);
    test_client!(get_deny: "GET", "/deny", results);
    test_client!(get_status_418: "GET", "/status/418", results);
    test_client!(post_status_201: "POST", "/status/201", results);
    test_client!(put_status_203: "PUT", "/status/203", results);
    test_client!(patch_status_201: "PATCH", "/status/201", results);
    test_client!(delete_status_200: "DELETE", "/status/200", results);
    println!();
}

fn run_server_tests(results: &mut TestResults) {
    if let Some(server) = build_and_start_server().as_mut() {
        println!("\n~~~~~~~~~~~~\nServer Tests\n~~~~~~~~~~~~");
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
        test_server!(connect_test: "CONNECT", "127.0.0.1:1234", results);
        println!();
        server.kill().unwrap();
    }
}

fn build_client() -> Option<()> {
    print!("Building client...");
    io::stdout().flush().unwrap();

    let build_status = Command::new("cargo")
        .args(["build", "-p", "client"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    if build_status.success() {
        println!("{GRN}✔{CLR}");
        Some(())
    } else {
        println!("{RED}✗{CLR}");
        None
    }
}

fn build_and_start_server() -> Option<Child> {
    let client_bin: PathBuf = [
        CRATE_ROOT,
        "..",
        "target",
        "debug",
        CLIENT_FILE
    ].iter().collect();

    print!("Building server...");
    io::stdout().flush().unwrap();

    let build_status = Command::new("cargo")
        .args(["build", "-p", "server"])
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

    let server_bin: PathBuf = [
        CRATE_ROOT,
        "..",
        "target",
        "debug",
        SERVER_FILE
    ].iter().collect();

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

        let output = Command::new(&client_bin)
            .args(["--server-tests", "--", "127.0.0.1:7878/"])
            .output()
            .unwrap();

        let res = String::from_utf8_lossy(&output.stdout);

        if successful_status(res.borrow()) {
            println!("{GRN}✔{CLR}");
            return Some(server);
        } else if attempt_num > max_attempts {
            println!("{RED}✗{CLR}\n\nServer took too long to go live.");
            server.kill().unwrap();
            return None;
        }
    }
}

fn successful_status(input: &str) -> bool {
    let input = input.trim_start();

    let Some((line, _)) = input.split_once('\n') else {
        return false;
    };

    let Ok(status_line) = StatusLine::parse(line) else {
        return false;
    };

    matches!(status_line.status.code(), 200..=299)
}

fn get_result(
    test_output: &[u8],
    expected_output: &PathBuf
) -> Option<(String, String)> {
    let output = String::from_utf8_lossy(test_output);
    let mut expected = String::new();

    let mut f = File::open(expected_output).unwrap();
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

    if output == expected {
        None
    } else {
        Some((output, expected))
    }
}

fn remove_old_builds(kind: &str) {
    print!("Removing old builds...");
    io::stdout().flush().unwrap();

    if kind == "client" || kind == "all" {
        let clean_client = Command::new("cargo")
            .args(["clean", "-p", "client"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();

        if clean_client.success() {
            print!("{GRN}✔ {CLR}");
        } else {
            print!("{RED}✗ {CLR}");
        }

        io::stdout().flush().unwrap();
    }

    if kind == "server" || kind == "all" {
        let clean_server = Command::new("cargo")
            .args(["clean", "-p", "server"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();

        if clean_server.success() {
            println!("{GRN}✔{CLR}");
        } else {
            println!("{RED}✗{CLR}");
        }
    } else {
        println!();
    }
}
