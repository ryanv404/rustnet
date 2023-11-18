use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Result as IoResult};
use std::process::{Command, Stdio};
use std::time::Instant;

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const YLW: &str = "\x1b[93m";
const CYAN: &str = "\x1b[96m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

#[test]
fn test_client() -> IoResult<()> {
    let mut results: BTreeMap<String, (String, String, f32)> = BTreeMap::new();

    let _ = Command::new("cargo")
        .args(["build", "--bin", "client"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()?;

    for entry in fs::read_dir("scripts/client_tests")? {
        let entry = entry?;
        let file = entry.file_name();
        let file = file.to_string_lossy();

        let Some((method, rest)) = file.split_once("_") else {
            eprintln!("Unable to parse method.");
            return Ok(());
        };

        let method = method.to_ascii_uppercase();

        let uri = match rest.rsplit_once(".") {
            Some((uri, _)) if uri == "jpeg" => "/image/jpeg".to_string(),
            Some((uri, _)) if uri == "png" => "/image/png".to_string(),
            Some((uri, _)) if uri == "svg" => "/image/svg".to_string(),
            Some((uri, _)) if uri == "webp" => "/image/webp".to_string(),
            Some((uri, _)) if uri == "utf8" => "/encoding/utf8".to_string(),
            Some((uri, _)) if uri == "text" => "/robots.txt".to_string(),
            Some((uri, _)) => format!("/{uri}"),
            None => {
                eprintln!("Unable to parse URI.");
                return Ok(());
            },
        };

        let label = format!("{method} {uri}");

        let now = Instant::now();

        let res = Command::new("target/debug/client")
            .args(["--testing", "httpbin.org", &uri])
            .output()?;

        let span = now.elapsed().as_secs_f32();

        let output = String::from_utf8_lossy(&res.stdout);

        let mut expected = String::new();
        let mut f = File::open(entry.path())?;
        f.read_to_string(&mut expected)?;

        let output = output.trim();
        let expected = expected.trim();

        results.insert(label, (output.to_string(), expected.to_string(), span));
    }

    println!("~~CLIENT TESTS~~");

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

    Ok(())
}
