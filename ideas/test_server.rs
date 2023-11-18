use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Result as IoResult};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const YLW: &str = "\x1b[93m";
const CYAN: &str = "\x1b[96m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

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

struct TestParams {
    method: String,
    uri: String,
    expected: String,
    span: f32,
}

fn parse_test_params_from_files(dir: &str) -> Vec<TestParams> {
    for entry in fs::read_dir(dir).unwrap();
        let entry = entry.unwrap();
        let path = entry.path();

        let file = entry.file_name();
        let file = file.to_string_lossy();

        let (method, uri) = match file.split_once('_').unwrap() {
            (method, s) if s == "index.txt" => {
                (method, String::from("/"))
            },
            (method, s) if s == "favicon.txt" => {
                (method, String::from("/favicon.ico"))
            },
            (method, s) if s.ends_with(".txt") => {
                let uri = String::from(s.strip_suffix(".txt"))

                if &*method == "connect" {
                    (method, uri)
                } else {
                }
            },
            (method, s) => {
                (method, format!("/{s}"))
            },
        };

        let method = method.to_ascii_uppercase();
}

#[test]
fn test_server() {
    let mut results: BTreeMap<String, (String, String, f32)> = BTreeMap::new();
        let label = format!("{method} {uri}");

        let now = Instant::now();

        let res = match &*method {
            "CONNECT" => Command::new("curl")
                .args([
                    "--silent",
                    "--include",
                    "-X", "CONNECT",
                    "-H", "Host: 127.0.0.1",
                    "--request-target", "127.0.0.1:7878",
                    "127.0.0.1:7878"
                ])
                .output()?,
            "HEAD" => Command::new("curl")
                .args([
                    "--silent",
                    "--include",
                    "--head",
                    &format!("127.0.0.1:7878{uri}")
                ])
                .output()?,
            _ => Command::new("curl")
                .args([
                    "--silent",
                    "--include",
                    "-X", &method,
                    &format!("127.0.0.1:7878{uri}")
                ])
                .output()?,
        };

        let span = now.elapsed().as_secs_f32();

        let output = String::from_utf8_lossy(&res.stdout);

        let mut expected = String::new();
        let mut f = File::open(entry.path())?;
        f.read_to_string(&mut expected)?;

        let output = output.trim();
        let expected = expected.trim();

        results.insert(label, (output.to_string(), expected.to_string(), span));
    }

    println!("~~SERVER TESTS~~");

    for (test, (out, exp, span)) in &results {
        let output = out
            .split("\n")
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>();

        let expected = exp
            .split("\n")
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>();

        let output = output.join("\n");
        let expected = expected.join("\n");

        if output == expected {
            println!("[{GRN}✔ PASSED{CLR} {span:.03}] {CYAN}{test}{CLR}");
        } else {
            println!("[{RED}✗ FAILED{CLR} {span:.03}] {CYAN}{test}{CLR}");
            println!("\n{YLW}--OUTPUT--\n{output}{CLR}");
            println!("{PURP}--EXPECTED--\n{expected}{CLR}\n");
        }
    }
}
