use std::process::Command;

// Trims ASCII whitespace bytes from both ends of a slice of bytes.
//
// Whitespace is: b'\t' (0x09), b'\n' (0x0a), b'\f' (0x0c), b'\r' (0x0d),
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
pub fn try_terminal_date() -> Option<String> {
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
