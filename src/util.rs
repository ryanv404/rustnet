use std::path::Path;
use std::process::{Command, Stdio};

use crate::{NetError, NetParseError, NetResult};

// Trims ASCII whitespace bytes from the start of a slice of bytes.
#[must_use]
pub fn trim_start_bytes(bytes: &[u8]) -> &[u8] {
    if bytes.is_empty() {
        return bytes;
    }

    // Find the index of the first non-whitespace byte.
    for i in 0..bytes.len() {
        if !bytes[i].is_ascii_whitespace() {
            return &bytes[i..];
        }
    }

    // The slice only contains whitespace.
    &[]
}

// Trims ASCII whitespace bytes from the end of a slice of bytes.
#[must_use]
pub fn trim_end_bytes(bytes: &[u8]) -> &[u8] {
    if bytes.is_empty() {
        return bytes;
    }

    // Find the index of the final non-whitespace byte.
    for i in (0..bytes.len()).rev() {
        if !bytes[i].is_ascii_whitespace() {
            return &bytes[..=i];
        }
    }

    // The slice only contains whitespace.
    &[]
}

// Trims ASCII whitespace bytes from both ends of a slice of bytes.
#[must_use]
pub fn trim_bytes(bytes: &[u8]) -> &[u8] {
    let trimmed = trim_start_bytes(bytes);
    trim_end_bytes(trimmed)
}

/// Builds the server binary using `cargo`.
/// 
/// # Errors
/// 
/// Returns an error if `cargo build` does not return an exit status of 0.
pub fn build_server() -> NetResult<()> {
    let mut build_handle = Command::new("cargo")
        .args(["build", "--bin", "server"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    match build_handle.wait() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(NetError::Other(format!("Status: {status}"))),
        Err(e) => Err(e.into()),
    }
}

/// Returns the file extension, if present, of a `Path` value.
#[must_use]
pub fn get_extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
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
                return Err(NetParseError::Path)?;
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
                "https" => Err(NetError::Https),
                _ => Err(NetParseError::Path)?,
            }
        },
        // No scheme present.
        None => match uri.split_once('/') {
            Some((addr, _)) if addr.is_empty() => {
                Err(NetParseError::Path)?
            },
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
            _ => Err(NetParseError::Path)?,
        },
    }
}

///// Get the current date and time if the `date` program exists.
//#[must_use]
//pub fn get_datetime() -> Option<(HeaderName, HeaderValue)> {
//    if !date_command_exists() {
//        return None;
//    }
//
//    Command::new("date")
//        .env("TZ", "GMT")
//        .arg("+%a, %d %b %Y %T %Z")
//        .output()
//        .map_or(None, |out| {
//            let trimmed = trim_bytes(&out.stdout);
//            let hdr_value = HeaderValue(trimmed.into());
//            Some((DATE, hdr_value))
//        })
//}
//
///// Returns true if the `date` exists on the system PATH.
//#[must_use]
//pub fn date_command_exists() -> bool {
//    let Ok(paths) = env::var("PATH") else {
//        return false;
//    };
//
//    for path in paths.split(':') {
//        if fs::metadata(format!("{path}/date")).is_ok() {
//            return true;
//        }
//    }
//
//    false
//}
