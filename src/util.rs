use std::process::Command;

// Trims ASCII whitespace bytes from both ends of a slice of bytes.
//
// Whitespace is b'\t' (0x09), b'\n' (0x0a), b'\f' (0x0c), b'\r' (0x0d),
// or b' ' (0x20).
#[must_use]
pub fn trim_whitespace_bytes(bytes: &[u8]) -> &[u8] {
    const EMPTY: &[u8; 0] = &[];

    let len = bytes.len();

    // Dispense with simple cases first.
    match len {
        1 if bytes[0].is_ascii_whitespace() => return EMPTY,
        0 | 1 => return bytes,
        _ => {}
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
    //
    // Note this could also be true if the original slice had size 1 but all
    // slices of size 0 or 1 were handled in the match statement above.
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
        // Only the final byte was non-whitespace.
        &bytes[first..=first]
    } else {
        // Multibyte slice remains.
        &bytes[first..=last]
    }
}

/// Attempt to use the terminal `date` program, if available, to get the
/// current date and time.
#[must_use]
pub fn try_date() -> Option<String> {
    if let Ok(date_out) = Command::new("date")
        .arg("--utc")
        .arg("+%a, %d %b %Y %H:%M:%S %Z")
        .output()
    {
        String::from_utf8(date_out.stdout)
            .map(|s| s.trim().replace("UTC", "GMT"))
            .ok()
    } else {
        None
    }
}

// Determines if byte is a token char.
#[allow(dead_code)]
#[must_use]
pub const fn is_token(b: u8) -> bool {
    b > 0x1F && b < 0x7F
}

// ASCII codes to accept URI string.
// i.e. A-Za-z0-9!#$%&'*+-._();:@=,/?[]~^
#[rustfmt::skip]
#[allow(dead_code)]
static URI_MAP: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0x0x
//  \0                            \n
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0x1x
//  commands
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0x2x
//  \w !  "  #  $  %  &  '  (  )  *  +  ,  -  .  /
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, // 0x3x
//  0  1  2  3  4  5  6  7  8  9  :  ;  <  =  >  ?
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0x4x
//  @  A  B  C  D  E  F  G  H  I  J  K  L  M  N  O
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0x5x
//  P  Q  R  S  T  U  V  W  X  Y  Z  [  \  ]  ^  _
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0x6x
//  `  a  b  c  d  e  f  g  h  i  j  k  l  m  n  o
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, // 0x7x
//  p  q  r  s  t  u  v  w  x  y  z  {  |  }  ~  del
//   ====== Extended ASCII (aka. obs-text) ======
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0x8x
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0x9x
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xAx
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xBx
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xCx
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xDx
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xEx
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xFx
];

#[allow(dead_code)]
#[must_use]
pub fn is_uri_token(b: u8) -> bool {
    URI_MAP[b as usize] == 1
}

#[rustfmt::skip]
#[allow(dead_code)]
static HEADER_NAME_MAP: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[allow(dead_code)]
#[must_use]
pub fn is_header_name_token(b: u8) -> bool {
    HEADER_NAME_MAP[b as usize] == 1
}

#[rustfmt::skip]
#[allow(dead_code)]
static HEADER_VALUE_MAP: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
];

#[allow(dead_code)]
#[must_use]
pub fn is_header_value_token(b: u8) -> bool {
    HEADER_VALUE_MAP[b as usize] == 1
}
