//! Translate owui-lint positions into LSP positions.
//!
//! The analyzer reports 1-indexed, **character-based** line/column pairs (a
//! column counts Unicode scalar values). LSP's default `PositionEncodingKind`
//! is **UTF-16**: `Position.character` counts UTF-16 code units. Characters
//! outside the Basic Multilingual Plane (most emoji, for instance) occupy two
//! UTF-16 units but a single `char`, and `str::find`/`rfind` return *byte*
//! offsets — so naively casting char counts or byte offsets to `u32` columns
//! misplaces ranges. These helpers keep every column the server emits in
//! UTF-16 units.

/// UTF-16 code-unit length of an entire line.
pub fn utf16_len(line: &str) -> u32 {
    line.encode_utf16().count() as u32
}

/// Convert a 0-indexed *character* offset within `line` into a UTF-16 code-unit
/// offset. An offset past the end of the line keeps its excess (treated as
/// single-unit), so a column pointing beyond the line content still yields a
/// sensible position.
pub fn char_to_utf16(line: &str, char_idx: usize) -> u32 {
    let mut units = 0u32;
    let mut chars = 0usize;
    for c in line.chars() {
        if chars >= char_idx {
            return units;
        }
        units += c.len_utf16() as u32;
        chars += 1;
    }
    units + (char_idx - chars) as u32
}

/// Convert a *byte* offset within `line` (e.g. from `str::find`/`rfind`) into a
/// UTF-16 code-unit offset. The offset is clamped to a char boundary so slicing
/// never panics on multi-byte input.
pub fn byte_to_utf16(line: &str, byte_idx: usize) -> u32 {
    let mut boundary = byte_idx.min(line.len());
    while boundary > 0 && !line.is_char_boundary(boundary) {
        boundary -= 1;
    }
    line[..boundary].encode_utf16().count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_matches_raw_counts() {
        let line = "def search():";
        assert_eq!(utf16_len(line), 13);
        assert_eq!(char_to_utf16(line, 4), 4);
        assert_eq!(byte_to_utf16(line, line.find("search").unwrap()), 4);
    }

    #[test]
    fn astral_char_counts_two_utf16_units() {
        // "🦀" is one `char`, four UTF-8 bytes, two UTF-16 code units.
        let line = "🦀 def x():";
        assert_eq!(utf16_len(line), "🦀 def x():".encode_utf16().count() as u32);
        // The char before "def" (crab + space) is 2 + 1 = 3 UTF-16 units.
        assert_eq!(char_to_utf16(line, 2), 3);
        // `find` returns a byte offset (crab is 4 bytes, space 1 -> 5).
        let byte = line.find("def").unwrap();
        assert_eq!(byte, 5);
        assert_eq!(byte_to_utf16(line, byte), 3);
    }

    #[test]
    fn char_offset_past_end_keeps_excess() {
        // Column pointing past a short/blank line keeps the extra columns.
        assert_eq!(char_to_utf16("", 2), 2);
        assert_eq!(char_to_utf16("ab", 5), 5);
    }

    #[test]
    fn byte_offset_clamps_to_char_boundary() {
        let line = "🦀x";
        // A byte offset landing inside the crab rounds down to its start (0).
        assert_eq!(byte_to_utf16(line, 2), 0);
        // Past the end clamps to the full UTF-16 length.
        assert_eq!(byte_to_utf16(line, 999), utf16_len(line));
    }
}
