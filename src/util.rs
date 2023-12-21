use std::path::Path;

use crate::{NetError, NetParseError, NetResult};

// Trims ASCII whitespace bytes from both ends of a slice of bytes.
#[must_use]
pub fn trim_whitespace_bytes(bytes: &[u8]) -> &[u8] {
    const EMPTY: &[u8; 0] = &[];

    let len = bytes.len();

    // Dispense with simple cases first.
    match len {
        1 if bytes[0].is_ascii_whitespace() => return EMPTY,
        0 | 1 => return bytes,
        _ => {},
    }

    let mut first: usize = 0;
    let mut last: usize = len - 1;
    let mut is_only_whitespace = true;

    // Find index of first non-whitespace byte.
    for (i, byte) in bytes.iter().enumerate().take(len) {
        if !byte.is_ascii_whitespace() {
            first = i;
            is_only_whitespace = false;
            break;
        }
    }

    // Slice only contains whitespace bytes.
    if is_only_whitespace {
        return EMPTY;
    }

    // Only the final byte was non-whitespace.
    if first == last {
        return &bytes[first..=first];
    }

    // Find index of last non-whitespace byte.
    for i in (first..=last).rev() {
        if !bytes[i].is_ascii_whitespace() {
            last = i;
            break;
        }
    }

    // Return trimmed slice
    if first == last {
        &bytes[first..=first]
    } else {
        &bytes[first..=last]
    }
}

/// Returns the file extension, if present, of a `Path` value.
#[must_use]
pub fn get_extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

/// Parses a string slice into a host address and a URI path.
#[allow(clippy::missing_errors_doc)]
pub fn parse_uri(uri: &str) -> NetResult<(String, String)> {
    let uri = uri.trim();

    if let Some((scheme, rest)) = uri.split_once("://") {
        // If "://" is present, we expect a URI like "http://httpbin.org".
        if scheme.is_empty() || rest.is_empty() {
            return Err(NetError::Parse(NetParseError::UriPath));
        }

        match scheme {
            "http" => match rest.split_once('/') {
                // Next "/" after the scheme, if present, starts the
                // path segment.
                Some((addr, path))
                    if path.is_empty() && addr.contains(':') =>
                {
                    // Example: http://httpbin.org:80/
                    Ok((addr.to_string(), String::from("/")))
                },
                Some((addr, path)) if path.is_empty() => {
                    // Example: http://httpbin.org/
                    Ok((format!("{addr}:80"), String::from("/")))
                },
                Some((addr, path)) if addr.contains(':') => {
                    // Example: http://httpbin.org:80/json
                    Ok((addr.to_string(), format!("/{path}")))
                },
                Some((addr, path)) => {
                    // Example: http://httpbin.org/json
                    Ok((format!("{addr}:80"), format!("/{path}")))
                },
                None if rest.contains(':') => {
                    // Example: http://httpbin.org:80
                    Ok((rest.to_string(), String::from("/")))
                },
                None => {
                    // Example: http://httpbin.org
                    Ok((format!("{rest}:80"), String::from("/")))
                },
            },
            "https" => Err(NetError::Https),
            _ => Err(NetError::Parse(NetParseError::UriPath)),
        }
    } else if let Some((addr, path)) = uri.split_once('/') {
        if addr.is_empty() {
            return Err(NetError::Parse(NetParseError::UriPath));
        }

        let addr = if addr.contains(':') {
            addr.to_string()
        } else {
            format!("{addr}:80")
        };

        let path = if path.is_empty() {
            String::from("/")
        } else {
            format!("/{path}")
        };

        Ok((addr, path))
    } else if uri.contains(':') {
        Ok((uri.to_string(), String::from("/")))
    } else {
        Ok((format!("{uri}:80"), String::from("/")))
    }
}

// pub fn is_file(path: &Path) -> bool {
//     fs::metadata(path)
//         .map(|meta| meta.is_file())
//         .unwrap_or(false)
// }

//// Determines if byte is a token char.
//#[allow(dead_code)]
//#[must_use]
//pub const fn is_token(b: u8) -> bool {
//    b > 0x1F && b < 0x7F
//}
//
//// ASCII codes to accept URI string.
//// i.e. A-Za-z0-9!#$%&'*+-._();:@=,/?[]~^
//#[rustfmt::skip]
//#[allow(dead_code)]
//static URI_MAP: [u8; 256] = [
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0x0x
////  \0                            \n
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0x1x
////  commands
//    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0x2x
////  \w !  "  #  $  %  &  '  (  )  *  +  ,  -  .  /
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, // 0x3x
////  0  1  2  3  4  5  6  7  8  9  :  ;  <  =  >  ?
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0x4x
////  @  A  B  C  D  E  F  G  H  I  J  K  L  M  N  O
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0x5x
////  P  Q  R  S  T  U  V  W  X  Y  Z  [  \  ]  ^  _
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0x6x
////  `  a  b  c  d  e  f  g  h  i  j  k  l  m  n  o
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, // 0x7x
////  p  q  r  s  t  u  v  w  x  y  z  {  |  }  ~  del
////   ====== Extended ASCII (aka. obs-text) ======
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0x8x
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0x9x
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xAx
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xBx
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xCx
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xDx
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xEx
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xFx
//];

///// Returns true if the given byte corresponds to a valid URI token.
//#[allow(dead_code)]
//#[must_use]
//pub fn is_uri_token(b: u8) -> bool {
//    URI_MAP[b as usize] == 1
//}
//
//#[rustfmt::skip]
//#[allow(dead_code)]
//static HEADER_NAME_MAP: [u8; 256] = [
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//    0, 1, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 0,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0,
//    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0,
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//];
//
///// Returns true if the given byte corresponds to a valid header name token.
//#[allow(dead_code)]
//#[must_use]
//pub fn is_header_name_token(b: u8) -> bool {
//    HEADER_NAME_MAP[b as usize] == 1
//}
//
//#[rustfmt::skip]
//#[allow(dead_code)]
//static HEADER_VALUE_MAP: [u8; 256] = [
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
//    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//];
//
///// Returns true if the given byte corresponds to a valid header value token.
//#[allow(dead_code)]
//#[must_use]
//pub fn is_header_value_token(b: u8) -> bool {
//    HEADER_VALUE_MAP[b as usize] == 1
//}
//
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
//            let trimmed = trim_whitespace_bytes(&out.stdout);
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

// HTTP Message Format:
//
// ( request-line / status-line ) CRLF
// *( field-line CRLF )
// CRLF
// [ message-body ]
//
// URI: scheme ":" "//" authority *(path) ["?" query] ["#" fragment]
// HTTP-URI: "http" "://" authority path-abempty [ "?" query ]
//
// authority     = host [":" port]
// host          = IP-literal / IPv4address / reg-name
// path          = *("/" *pchar)
// query         = ["?" *(pchar / "/" / "?")]
// fragment      = ["#" *(pchar / "/" / "?")]
// pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
// pct-encoded   = "%" HEXDIG HEXDIG
// unreserved    = ALPHA / DIGIT / "-" / "." / "_" / "~"
// reserved      = gen-delims / sub-delims
// gen-delims    = ":" / "/" / "?" / "#" / "[" / "]" / "@"
// sub-delims    = "!" / "$" / "&" / "'" / "(" / ")" /
//                 "*" / "+" / "," / ";" / "="
//
// start-line = request-line / status-line
// request-line = method SP request-target SP HTTP-version
// status-line = HTTP-version SP status-code SP [ reason-phrase ]