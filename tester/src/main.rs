use std::borrow::Borrow;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use librustnet::StatusLine;

const LOCAL_ADDR: &str = "127.0.0.1:7878";

const HTTPBIN_ADDR: &str = "httpbin.org:80";

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

            let client_bin: PathBuf = [
                CRATE_ROOT,
                "..",
                "target",
                "debug",
                CLIENT_FILE
            ].iter().collect();

            let output = Command::new(&client_bin)
                .args([
                    "--method",
                    $method,
                    "--path",
                    $uri_path,
                    "--client-tests",
                    "--",
                    HTTPBIN_ADDR
                ])
                .output()
                .unwrap();

            let output = get_trimmed_test_output(&output.stdout);

            let exp_file: PathBuf = [
                CRATE_ROOT,
                "..",
                "client",
                "test_data",
                concat!(stringify!($label), ".txt")
            ].iter().collect();

            let expected = get_expected_from_file(&exp_file);

            if output == expected {
                tracker.client_passed += 1;
                println!("[{GRN}✔{CLR}] {} {}", $method, $uri_path);
            } else {
                println!("[{RED}✗{CLR}] {} {}", $method, $uri_path);
                println!("OUTPUT:\n{output}\n\nEXPECTED:\n{expected}\n");
            }
        }

        $label($tracker)
    };
}

macro_rules! test_server {
    ($label:ident: $method:literal, $uri_path:literal, $tracker:expr) => {
        fn $label(tracker: &mut TestResults) {
            tracker.server_total += 1;

            let client_bin: PathBuf = [
                CRATE_ROOT,
                "..",
                "target",
                "debug",
                CLIENT_FILE
            ].iter().collect();

            let output = Command::new(&client_bin)
                .args([
                    "--method",
                    $method,
                    "--path",
                    $uri_path,
                    "--server-tests",
                    "--",
                    LOCAL_ADDR
                ])
                .output()
                .unwrap();

            let output = get_trimmed_test_output(&output.stdout);

            let expected = match $uri_path {
                "/many_methods" => {
                    let new_body = format!("Hi from the {} route!", $method);
                    get_expected_from_str(&new_body, $method)
                },
                _ => {
                    let exp_file: PathBuf = [
                        CRATE_ROOT,
                        "..",
                        "server",
                        "test_data",
                        concat!(stringify!($label), ".txt")
                    ].iter().collect();

                    get_expected_from_file(&exp_file)
                },
            };

            if output == expected {
                tracker.server_passed += 1;
                println!("[{GRN}✔{CLR}] {} {}", $method, $uri_path);
            } else {
                println!("[{RED}✗{CLR}] {} {}", $method, $uri_path);
                println!("OUTPUT:\n{output}\n\nEXPECTED:\n{expected}\n");
            }
        }

        $label($tracker)
    };
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
        test_server!(get_many_methods: "GET", "/many_methods", results);
        test_server!(post_many_methods: "POST", "/many_methods", results);
        test_server!(put_many_methods: "PUT", "/many_methods", results);
        test_server!(patch_many_methods: "PATCH", "/many_methods", results);
        test_server!(delete_many_methods: "DELETE", "/many_methods", results);
        test_server!(trace_many_methods: "TRACE", "/many_methods", results);
        test_server!(options_many_methods: "OPTIONS", "/many_methods", results);
        test_server!(connect_many_methods: "CONNECT", "/many_methods", results);
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
            .args(["--server-tests", "--", LOCAL_ADDR])
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
    match input.trim_start().split_once('\n') {
        Some((line, _)) => match StatusLine::parse(line) {
            Ok(status_line) => matches!(status_line.status.code(), 200..=299),
            _ => false,
        },
        _ => false,
    }
}

fn get_trimmed_test_output(output: &[u8]) -> String {
    let output_str = String::from_utf8_lossy(output);

    output_str
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();
            if !line.is_empty() && line.starts_with("Host:") {
                if let Some((name, _old_value)) = line.split_once(':') {
                    let current_host = format!("{name}: {HTTPBIN_ADDR}");
                    Some(current_host)
                } else {
                    Some(line.to_string())
                }
            } else if !line.is_empty() {
                Some(line.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn get_expected_from_str(body: &str, method: &str) -> String {
    let status = if method == "POST" {
        "201 Created"
    } else {
        "200 OK"
    };

    format!("\
        HTTP/1.1 {}\n\
        Cache-Control: no-cache\n\
        Content-Length: {}\n\
        Content-Type: text/plain; charset=utf-8\n\
        Server: rustnet/0.1.0\n\
        {}",
        status,
        body.len(),
        body
    )
}

fn get_expected_from_file(exp_file: &Path) -> String {
    let mut exp_output = String::new();
    let mut f = File::open(exp_file).unwrap();
    f.read_to_string(&mut exp_output).unwrap();

    exp_output
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();
            if !line.is_empty() && line.starts_with("Host:") {
                if let Some((name, _old_value)) = line.split_once(':') {
                    let current_host = format!("{name}: {HTTPBIN_ADDR}");
                    Some(current_host)
                } else {
                    Some(line.to_string())
                }
            } else if !line.is_empty() {
                Some(line.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
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
