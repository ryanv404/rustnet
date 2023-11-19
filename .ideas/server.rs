use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Result as IoResult};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn start_test_server() -> Child {
    let _ = Command::new("cargo")
        .args(["build", "--bin", "server"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap();

    let server = Command::new("target/debug/server")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    thread::sleep(Duration::from_secs(2));

    server
}

fn stop_test_server(server: Child) {
    server.kill().unwrap();
}

#[derive(Debug, Clone)]
struct TestConfig {
    method: String,
    uri: String,
    file_path: String,
}

fn get_test_configs(dir: &str) -> Vec<TestConfig> {
    let mut tests: Vec<TestConfig> = vec![];

    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let file = entry.file_name();
        let file = file.to_string_lossy();

        let (method, uri) = match file.split_once('_').unwrap() {
            (method, s) if s == "index.txt" => {
                (method, String::from("/"))
            },
            (method, s) if s == "favicon.txt" => {
                (method, String::from("/favicon.ico"))
            },
            (method, s) => {
                if s.ends_with(".txt") {
                    (method, format!("/{}", s.strip_suffix(".txt")))
                } else {
                    (method, format!("/{s}"))
                }
            },
        };

        let file_path = entry.path();
        let method = method.to_ascii_uppercase();

        tests.push(TestConfig { method, uri, file_path })
    }
}

//GET /about
//GET /foo
//GET /
//HEAD /
//HEAD /about
//HEAD /foo
//HEAD /favicon.ico
//POST /about
//PUT /about
//PATCH /about
//DELETE /about
//TRACE /about
//OPTIONS /about

macro_rules! test_server_route {
    ($label:ident: $method:literal $uri:literal) => {
        #[test]
        fn $label() {
            let res = match &*method {
                "HEAD" => Command::new("curl")
                    .args([
                        "--silent",
                        "--include",
                        "--head",
                        &format!("127.0.0.1:7878{}", $uri)
                    ])
                    .output()
                    .unwrap(),
                _ => Command::new("curl")
                    .args([
                        "--silent",
                        "--include",
                        "-X", $method,
                        &format!("127.0.0.1:7878{}", $uri)
                    ])
                    .output()
                    .unwrap(),
            };

            let output = String::from_utf8_lossy(&res.stdout);
            let output = output
                .split('\n')
                .map(|s| s.trim().to_string())
                .collect::<Vec<String>>();
            let output = output.join('\n');

            let mut expected = String::new();
            let mut f = File::open(entry.path()).unwrap();
            f.read_to_string(&mut expected).unwrap();

            let expected = expected
                .split('\n')
                .map(|s| s.trim().to_string())
                .collect::<Vec<String>>();
            let expected = expected.join('\n');

            assert_eq!(output, expected);
        }
    };
}
