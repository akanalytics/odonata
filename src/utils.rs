use std::ops::{Bound, RangeBounds};

//
// https://users.rust-lang.org/t/how-to-get-a-substring-of-a-string/1351/9
//
pub trait StringUtils {
    fn take_substring(&self, start: usize, len: usize) -> &str;
    fn take_slice(&self, range: impl RangeBounds<usize>) -> &str;
    fn take_char_at(&self, i: usize) -> Option<char>;
}

impl StringUtils for str {
    fn take_substring(&self, start: usize, len: usize) -> &str {
        let mut char_pos = 0;
        let mut byte_start = 0;
        let mut it = self.chars();
        loop {
            if char_pos == start {
                break;
            }
            if let Some(c) = it.next() {
                char_pos += 1;
                byte_start += c.len_utf8();
            } else {
                break;
            }
        }
        char_pos = 0;
        let mut byte_end = byte_start;
        loop {
            if char_pos == len {
                break;
            }
            if let Some(c) = it.next() {
                char_pos += 1;
                byte_end += c.len_utf8();
            } else {
                break;
            }
        }
        &self[byte_start..byte_end]
    }
    fn take_slice(&self, range: impl RangeBounds<usize>) -> &str {
        let start = match range.start_bound() {
            Bound::Included(bound) | Bound::Excluded(bound) => *bound,
            Bound::Unbounded => 0,
        };
        let len = match range.end_bound() {
            Bound::Included(bound) => *bound + 1,
            Bound::Excluded(bound) => *bound,
            Bound::Unbounded => self.len(),
        } - start;
        self.take_substring(start, len)
    }

    fn take_char_at(&self, i: usize) -> Option<char> {
        self.chars().nth(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn string_utils() {
        let s = "abcdèfghij";
        assert_eq!(s.take_char_at(50), None);
        // All three statements should print:
        // "abcdè, abcdèfghij, dèfgh, dèfghij, g."
        let expected = "abcdè, abcdèfghij, dèfgh, dèfghij, g.";
        assert_eq!(
            expected,
            format!(
                "{}, {}, {}, {}, {}.",
                s.take_substring(0, 5),
                s.take_substring(0, 50),
                s.take_substring(3, 5),
                s.take_substring(3, 50),
                s.take_char_at(6).unwrap()
            )
        );
        assert_eq!(
            expected,
            format!(
                "{}, {}, {}, {}, {}.",
                s.take_slice(..5),
                s.take_slice(..50),
                s.take_slice(3..8),
                s.take_slice(3..),
                s.take_char_at(6).unwrap()
            )
        );
        assert_eq!(
            expected,
            format!(
                "{}, {}, {}, {}, {}.",
                s.take_slice(..=4),
                s.take_slice(..=49),
                s.take_slice(3..=7),
                s.take_slice(3..),
                s.take_char_at(6).unwrap()
            )
        );
    }
}
