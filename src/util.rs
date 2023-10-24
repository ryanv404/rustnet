use std::slice;

// Trims ASCII whitespace bytes from both ends of a slice of bytes.
//
// Whitespace is: b'\t' (0x09), b'\n' (0x0a), b'\f' (0x0c), b'\r' (0x0d),
// or b' ' (0x20).
#[must_use]
pub fn trim_whitespace(bytes: &[u8]) -> &[u8] {
    let len = bytes.len();

    // Dispense with simple cases first.
    match len {
        0 => return bytes,
        1 if bytes[0].is_ascii_whitespace() => return b"",
        1 => return bytes,
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
        return b"";
    }

    // Only the final byte was non-whitespace.
    //
    // Note this could also be true if the original slice had size 1 but all
    // slices of size 0 or 1 were handled in the match statement above.
    if first == last {
        return slice::from_ref(&bytes[first]);
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
        slice::from_ref(&bytes[first])
    } else {
        // Multibyte slice remains.
        &bytes[first..=last]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_whitespace_trimming {
        ($( $name:ident: $input:expr => $expected:expr, )+) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!(trim_whitespace($input), $expected);
                }
            )+
        };
    }

    test_whitespace_trimming! {
        trim_start: b"  test" => b"test",
        trim_end: b"test    " => b"test",
        trim_both_sides: b"         test       " => b"test",
        trim_both_sides_not_inner: b"  Hello \nworld       " => b"Hello \nworld",
        trim_mutliple_whitespace_types: b"\t  \nx\t  x\r\x0c" => b"x\t  x",
        trim_many_whitespaces: b"                   " => b"",
        trim_single_whitespace: b" " => b"",
        trim_single_non_whitespace: b"x" => b"x",
        trim_empty_slice: b"" => b"",
    }
}
