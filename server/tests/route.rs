use std::borrow::Borrow;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use librustnet::StatusLine;

const LOCAL_ADDR: &str = "127.0.0.1:7878";
const HTTPBIN_ADDR: &str = "httpbin.org:80";
const ROOT: &str = env!("CARGO_MANIFEST_DIR");

#[cfg(not(windows))]
const CLIENT_FILE: &str = "client";
#[cfg(windows)]
const CLIENT_FILE: &str = "client.exe";

#[cfg(not(windows))]
const SERVER_FILE: &str = "server";
#[cfg(windows)]
const SERVER_FILE: &str = "server.exe";

macro_rules! build_client {
    () => {{
        let build_status = Command::new("cargo")
            .args(["build", "-p", "client"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();

        assert!(build_status.success());
    }};
}

macro_rules! start_server {
    () => {{
        let build_status = Command::new("cargo")
            .args(["build", "-p", "server"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();

        assert!(build_status.success());

        let server_bin: PathBuf = [
            ROOT,
            "..",
            "target",
            "debug",
            SERVER_FILE
        ].iter().collect();

        let server = Command::new(server_bin.clone())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let client_bin: PathBuf = [
            ROOT,
            "..",
            "target",
            "debug",
            CLIENT_FILE
        ].iter().collect();

        let mut attempt_num = 0;
        let mut server_is_live = false;

        while attempt_num < 3 {
            thread::sleep(Duration::from_millis(500));

            let output = Command::new(&client_bin)
                .args(["--server-tests", "--", LOCAL_ADDR])
                .output()
                .unwrap();

            let res = String::from_utf8_lossy(&output.stdout);

            if successful_status(res.borrow()) {
                server_is_live = true;
                break;
            } else {
                attempt_num += 1;
            }
        }

        assert!(
            server_is_live,
            "Server took too long to go live.\n{}",
            &server_bin.display()
        );

        server
    }};
}

macro_rules! run_server_test {
    ($label:ident: $method:literal, $uri_path:literal) => {
        fn $label() {
            let client_bin: PathBuf = [
                ROOT,
                "..",
                "target",
                "debug",
                CLIENT_FILE
            ].iter().collect();

            let output = Command::new(&client_bin)
                .args([
                    "--method", $method,
                    "--path", $uri_path,
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
                        ROOT,
                        "test_data",
                        concat!(stringify!($label), ".txt")
                    ].iter().collect();

                    get_expected_from_file(&exp_file)
                },
            };

            assert_eq!(output, expected);
        }

        $label();
    };
}

macro_rules! kill_server {
    ($server:expr) => { $server.kill().unwrap(); };
}

macro_rules! run_server_tests {
    ( $($label:ident: $method:literal, $uri_path:literal;)+ ) => {
        #[test]
        fn run_tests() {
            let mut server = start_server!();
            $(
                run_server_test!($label: $method, $uri_path);
            )+
            kill_server!(server);
        }
    };
}

fn successful_status(input: &str) -> bool {
    match input.trim_start().split_once('\n') {
        Some((line, _)) => match StatusLine::parse(line) {
            Ok(status_line) => {
                matches!(status_line.status.code(), 200..=299)
            },
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

            if line.starts_with("Host:") {
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

            if line.starts_with("Host:") {
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

// Server tests.
run_server_tests! {
    get_about: "GET", "/about";
    get_foo: "GET", "/foo";
    get_index: "GET", "/";
    head_index: "HEAD", "/";
    head_about: "HEAD", "/about";
    head_foo: "HEAD", "/foo";
    head_favicon: "HEAD", "/favicon.ico";
    get_many_methods: "GET", "/many_methods";
    post_many_methods: "POST", "/many_methods";
    put_many_methods: "PUT", "/many_methods";
    patch_many_methods: "PATCH", "/many_methods";
    delete_many_methods: "DELETE", "/many_methods";
    trace_many_methods: "TRACE", "/many_methods";
    options_many_methods: "OPTIONS", "/many_methods";
    connect_many_methods: "CONNECT", "/many_methods";
}
