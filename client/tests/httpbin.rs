use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

macro_rules! run_client_test {
    ($label:ident: $method:literal, $uri_path:literal) => {
        #[test]
        fn $label() {
            let output = Command::new("cargo")
                .args([
                    "run",
                    "-p", "client",
                    "--",
                    "--method", $method,
                    "--path", $uri_path,
                    "--client-tests",
                    "--",
                    "httpbin.org:80"
                ])
                .output()
                .unwrap();

            let output = get_trimmed_test_output(&output.stdout);

            let filename = format!(
                "{}_{}.txt",
                $method.to_lowercase(),
                stringify!($label)
            );

            let exp_file: PathBuf = [
                ROOT,
                "test_data",
                &filename
            ].iter().collect();

            let expected = get_expected_from_file(&exp_file);

            assert_eq!(output, expected, "Failure at: {}", stringify!($label));
        }
    };
}

fn get_trimmed_test_output(output: &[u8]) -> String {
    let output_str = String::from_utf8_lossy(output);

    output_str
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();

            if line.starts_with("Host:") {
                if let Some((name, _old_value)) = line.split_once(':') {
                    let current_host = format!("{name}: httpbin.org:80");
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
                    let current_host = format!("{name}: httpbin.org:80");
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

// Client tests
mod get {
    use super::*;
    run_client_test!(deny: "GET", "/deny");
    run_client_test!(html: "GET", "/html");
    run_client_test!(json: "GET", "/json");
    run_client_test!(xml: "GET", "/xml");
    run_client_test!(robots_txt: "GET", "/robots.txt");
    run_client_test!(encoding_utf8: "GET", "/encoding/utf8");
    run_client_test!(image_jpeg: "GET", "/image/jpeg");
    run_client_test!(image_png: "GET", "/image/png");
    run_client_test!(image_svg: "GET", "/image/svg");
    run_client_test!(image_webp: "GET", "/image/webp");
    run_client_test!(status_418: "GET", "/status/418");
}

mod post {
    use super::*;
    run_client_test!(status_201: "POST", "/status/201");
}

mod patch {
    use super::*;
    run_client_test!(status_201: "PATCH", "/status/201");
}

mod put {
    use super::*;
    run_client_test!(status_203: "PUT", "/status/203");
}

mod delete {
    use super::*;
    run_client_test!(status_200: "DELETE", "/status/200");
}
