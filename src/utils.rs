use std::env;
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use crate::{HeaderValue, NetError, NetResult};
use crate::style::colors::{RED, RESET};

/// Trim whitespace from the beginning of a bytes slice.
#[must_use]
pub const fn trim_start(mut bytes: &[u8]) -> &[u8] {
    while let [first, rest @ ..] = bytes {
        if first.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }

    bytes
}

/// Trim whitespace from the end of a bytes slice.
#[must_use]
pub const fn trim_end(mut bytes: &[u8]) -> &[u8] {
    while let [rest @ .., last] = bytes {
        if last.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }

    bytes
}

/// Trim whitespace from the beginning and the end of a bytes slice.
#[must_use]
pub const fn trim(bytes: &[u8]) -> &[u8] {
    trim_end(trim_start(bytes))
}

/// Parses a string slice into a host address and a URI path.
/// 
/// # Errors
/// 
/// Returns an error if the `uri` argument cannot be parsed into an address
/// `String` and a path `String`.
pub fn parse_uri(uri: &str) -> NetResult<(String, String)> {
    match uri.trim().split_once("://") {
        Some((scheme, rest)) => {
            if scheme.is_empty() || rest.is_empty() {
                return Err(NetError::BadUri);
            }

            match scheme {
                "http" => match rest.split_once('/') {
                    Some((addr, path)) => {
                        if path.is_empty() && addr.contains(':') {
                            // http://httpbin.org:80/
                            Ok((addr.to_string(), String::from("/")))
                        } else if path.is_empty() {
                            // http://httpbin.org/
                            Ok((format!("{addr}:80"), String::from("/")))
                        } else if addr.contains(':') {
                            // http://httpbin.org:80/json
                            Ok((addr.to_string(), format!("/{path}")))
                        } else {
                            // http://httpbin.org/json
                            Ok((format!("{addr}:80"), format!("/{path}")))
                        }
                    },
                    None if rest.contains(':') => {
                        // http://httpbin.org:80
                        Ok((rest.to_string(), String::from("/")))
                    },
                    // http://httpbin.org
                    None => Ok((format!("{rest}:80"), String::from("/"))),
                },
                "https" => Err(NetError::HttpsNotImplemented),
                _ => Err(NetError::BadScheme),
            }
        },
        // No scheme present.
        None => match uri.split_once('/') {
            Some((addr, _)) if addr.is_empty() => Err(NetError::BadAddress),
            Some((addr, path)) if addr.contains(':') && path.is_empty() => {
                // httpbin.org:80/
                Ok((addr.to_string(), String::from("/")))
            },
            Some((addr, path)) if addr.contains(':') => {
                // httpbin.org:80/json
                Ok((addr.to_string(), format!("/{path}")))
            },
            Some((addr, path)) if path.is_empty() => {
                // httpbin.org/
                Ok((format!("{addr}:80"), String::from("/")))
            },
            Some((addr, path)) => {
                // httpbin.org/json
                Ok((format!("{addr}:80"), format!("/{path}")))
            },
            None if uri.contains(':') => {
                // httpbin.org:80
                Ok((uri.to_string(), String::from("/")))
            },
            None if uri.contains('.') => {
                // httpbin.org
                Ok((format!("{uri}:80"), String::from("/")))
            },
            _ => Err(NetError::BadUri),
        },
    }
}

/// Converts the given string slice to a new titlecase `String`.
#[must_use]
pub fn to_titlecase(input: &str) -> String {
    let mut output = String::with_capacity(input.len());

    let parts = input.trim().split('-').collect::<Vec<&str>>();

    for (part_idx, part) in parts.into_iter().enumerate() {
        if part_idx != 0 {
            // Restore any hyphens that were removed by `split`.
            output.push('-');
        }

        for (c_idx, c) in part.chars().enumerate() {
            // The first letter of each part should be uppercase.
            if c_idx == 0 {
                if c.is_ascii_lowercase() {
                    let upper = c.to_ascii_uppercase();
                    output.push(upper);
                } else {
                    output.push(c);
                }

                continue;
            }

            // All other letters should be lowercase.
            if c.is_ascii_uppercase() {
                let lower = c.to_ascii_lowercase();
                output.push(lower);
            } else {
                output.push(c);
            }
        }
    }

    output
}

/// Builds the server binary using `cargo`.
/// 
/// # Errors
/// 
/// Returns an error if `cargo build` does not return an exit status of 0.
pub fn build_server() -> NetResult<()> {
    let mut build_handle = match Command::new("cargo")
        .args(["build", "--bin", "server"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(handle) => handle,
        Err(e) => {
            eprintln!("{RED}Error while spawning cargo build.{RESET}");
            return Err(e.into());
        },
    };

    match build_handle.wait() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => {
            let msg = format!("Status: {status}");
            Err(NetError::Other(msg.into()))
        },
        Err(e) => {
            eprintln!("{RED}Error while waiting for build to finish.{RESET}");
            Err(e.into())
        },
    }
}

/// Get the current date and time if the `date` program exists on Unix.
#[must_use]
pub fn get_datetime() -> Option<HeaderValue> {
    let Some(date_path) = date_command_exists() else {
        return None;
    };

    Command::new(date_path)
        .env("TZ", "GMT")
        .arg("+%a, %d %b %Y %T %Z")
        .output()
        .map_or(None, |out| Some(HeaderValue(trim(&out.stdout).into())))
}

/// Returns the path to the `date` program if it exists.
#[must_use]
pub fn date_command_exists() -> Option<String> {
    if !cfg!(unix) {
        return None;
    }

    let Ok(paths) = env::var("PATH") else {
        return None;
    };

    for path in paths.split(':') {
        let path = format!("{path}/date");

        if fs::metadata(&path).is_ok() {
            return Some(path);
        }
    }

    None
}

/// Returns true if a TCP connection can be established with the provided
/// server address.
#[must_use]
pub fn check_server_is_live(addr: &str) -> bool {
    let Ok(socket) = SocketAddr::from_str(addr) else {
        return false;
    };

    let timeout = Duration::from_millis(200);

    // Attempt to connect a maximum of five times.
    for _ in 0..5 {
        if TcpStream::connect_timeout(&socket, timeout).is_ok() {
            return true;
        }

        thread::sleep(timeout);
    }

    false
}

/// Returns the file extension, if present, of a `Path` value.
#[must_use]
pub fn get_extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

/// Returns the Content-Type header value from a file extension, if possible.
#[must_use]
pub fn content_type_from_ext(path: &Path) -> Option<&'static str> {
    match get_extension(path) {
        Some("gif") => Some("image/gif"),
        Some("html" | "htm") => Some("text/html; charset=utf-8"),
        Some("ico") => Some("image/x-icon"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("json") => Some("application/json"),
        Some("pdf") => Some("application/pdf"),
        Some("png") => Some("image/png"),
        Some("txt") => Some("text/plain; charset=utf-8"),
        Some("xml") => Some("application/xml"),
        _ => None,
    }
}
