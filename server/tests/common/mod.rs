pub const LOCAL_ADDR: &str = "127.0.0.1:7878";

pub const CONNECT_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 26
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the CONNECT route!";

pub const DELETE_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 25
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the DELETE route!";

pub const GET_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 22
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the GET route!";

pub const GET_ABOUT: &str = r#"
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 455
    Content-Type: text/html; charset=utf-8
    Server: rustnet/0.1.0

    <!DOCTYPE html>
    <html lang="en">
        <head>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <title>About</title>
        </head>
        <body style="background-color:black;">
            <main style="color:white;">
                <h2 style="text-align:left;">Hi. I'm Ryan.</h2>
                <p><a href="/" style="color:lightgray; text-decoration:none;">Home</a></p>
            </main>
        </body>
    </html>"#;

pub const GET_FOO: &str = r#"
    HTTP/1.1 404 Not Found
    Cache-Control: no-cache
    Content-Length: 482
    Content-Type: text/html; charset=utf-8
    Server: rustnet/0.1.0

    <!DOCTYPE html>
    <html lang="en">
        <head>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <title>Not Found</title>
        </head>
        <body style="background-color:black;">
            <main style="color:white;">
                <h2 style="text-align:left;">Sorry, that page could not be found.</h2>
                <p><a href="/" style="color:lightgray; text-decoration:none;">Home</a></p>
            </main>
        </body>
    </html>"#;

pub const GET_INDEX: &str = r#"
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 575
    Content-Type: text/html; charset=utf-8
    Server: rustnet/0.1.0

    <!DOCTYPE html>
    <html lang="en">
        <head>
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <meta charset="utf-8">
            <title>Home</title>
        </head>
        <body style="background-color:black;">
            <main style="color:white;">
                <h1 style="text-align:center; padding:10px;">Welcome home.</h1>
                <h2>Links:</h2>
                <ul style="list-style-type:none;">
                    <li><a href="/about" style="color:lightgray; text-decoration:none;">About</a></li>
                </ul>
            </main>
        </body>
    </html>"#;

pub const HEAD_ABOUT: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 455
    Content-Type: text/html; charset=utf-8
    Server: rustnet/0.1.0";

pub const HEAD_FAVICON: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: max-age=604800
    Content-Length: 1406
    Content-Type: image/x-icon
    Server: rustnet/0.1.0";

pub const HEAD_FOO: &str = "\
    HTTP/1.1 404 Not Found
    Cache-Control: no-cache
    Content-Length: 482
    Content-Type: text/html; charset=utf-8
    Server: rustnet/0.1.0";

pub const HEAD_INDEX: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 575
    Content-Type: text/html; charset=utf-8
    Server: rustnet/0.1.0";

pub const HEAD_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 22
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0";

pub const OPTIONS_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 26
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the OPTIONS route!";

pub const PATCH_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 24
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the PATCH route!";

pub const POST_MANY_METHODS: &str = "\
    HTTP/1.1 201 Created
    Cache-Control: no-cache
    Content-Length: 23
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the POST route!";

pub const PUT_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 22
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the PUT route!";

pub const TRACE_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 24
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the TRACE route!";

pub fn is_server_live(addr: &str) -> bool {
    use std::net::TcpStream;

    if TcpStream::connect(addr).is_ok() {
        true
    } else {
        false
    }
}

pub fn get_test_output(input: &str) -> String {
    input
        .trim()
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();

            if line.is_empty() {
                None
            } else {
                Some(line.to_string())
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn get_expected_output(method: &str, path: &str) -> String {
    let output = match method {
        "GET" => match path {
            "/" => GET_INDEX,
            "/about" => GET_ABOUT,
            "/foo" => GET_FOO,
            "/many_methods" => GET_MANY_METHODS,
            _ => unreachable!(),
        },
        "HEAD" => match path {
            "/" => HEAD_INDEX,
            "/about" => HEAD_ABOUT,
            "/favicon.ico" => HEAD_FAVICON,
            "/foo" => HEAD_FOO,
            "/many_methods" => HEAD_MANY_METHODS,
            _ => unreachable!(),
        },
        "POST" => POST_MANY_METHODS,
        "PUT" => PUT_MANY_METHODS,
        "PATCH" => PATCH_MANY_METHODS,
        "DELETE" => DELETE_MANY_METHODS,
        "TRACE" => TRACE_MANY_METHODS,
        "OPTIONS" => OPTIONS_MANY_METHODS,
        "CONNECT" => CONNECT_MANY_METHODS,
        _ => unreachable!(),
    };

    output
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();

            if line.is_empty() {
                None
            } else {
                Some(line.to_string())
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

macro_rules! run_server_tests {
    (START_TEST_SERVER) => {
        #[test]
        fn test_server_started() {
            use std::process::{Command, Stdio};
            use std::thread;
            use std::time::Duration;
            use $crate::common::{is_server_live, LOCAL_ADDR};

            let _server = Command::new("cargo")
                .args([
                    "run",
                    "-p", "server",
                    "--",
                    "--use-shutdown-route"
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();

            let mut attempt_num = 0;

            while attempt_num < 5 {
                if is_server_live(LOCAL_ADDR) {
                    return;
                } else {
                    thread::sleep(Duration::from_millis(300));
                    attempt_num += 1;
                }
            }

            panic!("Server took too long to go live.");
        }
    };
    (SHUTDOWN_TEST_SERVER) => {
        #[test]
        fn test_server_shutdown() {
            use std::process::{Command, Stdio};
            use std::thread;
            use std::time::Duration;
            use $crate::common::{is_server_live, LOCAL_ADDR};

            let _shutdown = Command::new("cargo")
                .args([
                    "run",
                    "-p", "client",
                    "--",
                    "--method", "DELETE",
                    "--path", "/__shutdown_server__",
                    "--server-tests",
                    "--",
                    LOCAL_ADDR
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();

            let mut attempt_num = 0;

            while attempt_num < 5 {
                if is_server_live(LOCAL_ADDR) {
                    thread::sleep(Duration::from_millis(200));
                    attempt_num += 1;
                } else {
                    break;
                }
            }

            assert!(!is_server_live(LOCAL_ADDR));
        }
    };
    ($( $label:ident: $method:literal, $uri_path:literal; )+) => {
        $(
            #[test]
            fn $label() {
                use std::process::Command;
                use $crate::common::{
                    get_expected_output, get_test_output, LOCAL_ADDR,
                };

                let output = Command::new("cargo")
                    .args([
                        "run",
                        "-p", "client",
                        "--",
                        "--method", $method,
                        "--path", $uri_path,
                        "--server-tests",
                        "--",
                        LOCAL_ADDR
                    ])
                    .output()
                    .unwrap();

                let output_str = String::from_utf8(output.stdout).unwrap();
                let output = get_test_output(&output_str);
                let expected = get_expected_output($method, $uri_path);

                assert_eq!(output, expected);
            }
        )+
    };
}
