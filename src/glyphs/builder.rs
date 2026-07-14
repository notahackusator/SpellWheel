use std::collections::BTreeSet;

pub fn build_glyph_ranges(chars: impl Iterator<Item = char>) -> Vec<u32> {
    let mut codepoints = BTreeSet::new();
    for c in chars {
        let cp = c as u32;
        if cp <= 0xFFFF {
            codepoints.insert(cp);
        }
    }

    let mut ranges = Vec::new();
    let mut iter = codepoints.into_iter().peekable();
    while let Some(start) = iter.next() {
        let mut end = start;
        while let Some(&next) = iter.peek() {
            if next == end + 1 {
                end = next;
                iter.next();
            } else {
                break;
            }
        }
        ranges.push(start);
        ranges.push(end);
    }
    ranges.push(0);
    ranges
}