#![allow(unused)]

use core::{ptr::null, slice, str::from_utf8_unchecked};

/// Similar to [`core::str::Chars`] but it can peek and retain pointer information
pub struct Chars<'a> {
    src: &'a str,
    iter: slice::Iter<'a, u8>,
    ptr: *const u8,
    ch: Option<char>,
}

/// Points to a valid UTF8 character inside a [`str`], used to take sub strings
#[derive(Copy, Clone)]
pub struct Cursor {
    ptr: *const u8,
}

impl<'a> Chars<'a> {
    #[inline]
    pub fn offset_from_source_str(&self) -> usize {
        unsafe { self.ptr.offset_from(self.src.as_ptr()) as _ }
    }

    #[inline]
    pub fn cursor(&self) -> Cursor {
        Cursor { ptr: self.ptr }
    }

    #[inline]
    pub fn head(&self) -> Option<char> {
        self.ch
    }

    #[inline]
    pub fn sub_str_from_cursor(&self, cursor: Cursor) -> &'a str {
        unsafe {
            let offset = cursor.ptr.offset_from(self.src.as_ptr());
            assert!(
                0 <= offset && (offset as usize) < self.src.len(),
                "cursor is from a different str"
            );
            from_utf8_unchecked(slice::from_raw_parts(
                cursor.ptr,
                self.ptr.offset_from(cursor.ptr) as _,
            ))
        }
    }

    #[inline]
    pub fn source(&self) -> &'a str {
        self.src
    }

    #[must_use]
    #[inline]
    pub fn tail(&self) -> &'a str {
        // SAFETY: `Chars` is only made from a str, which guarantees the iter is valid UTF-8.
        unsafe { from_utf8_unchecked(self.iter.as_slice()) }
    }
}

impl<'a> From<&'a str> for Chars<'a> {
    fn from(src: &'a str) -> Self {
        let mut chars = Chars {
            src,
            iter: src.as_bytes().iter(),
            ptr: null(),
            ch: None,
        };
        chars.next();
        chars
    }
}

impl<'a> Iterator for Chars<'a> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        let tmp = self.ch;
        self.ptr = self.iter.as_slice().as_ptr();
        // SAFETY: `str` invariant says `self.iter` is a valid UTF-8 string and
        // the resulting `ch` is a valid Unicode Scalar Value.
        self.ch = next_code_point(&mut self.iter).map(|ch| unsafe { char::from_u32_unchecked(ch) });
        tmp
    }
}

#[inline]
fn next_code_point<'a, I: Iterator<Item = &'a u8>>(bytes: &mut I) -> Option<u32> {
    /// Returns the initial codepoint accumulator for the first byte.
    /// The first byte is special, only want bottom 5 bits for width 2, 4 bits
    /// for width 3, and 3 bits for width 4.
    #[inline]
    const fn utf8_first_byte(byte: u8, width: u32) -> u32 {
        (byte & (0x7F >> width)) as u32
    }

    /// Returns the value of `ch` updated with continuation byte `byte`.
    #[inline]
    const fn utf8_acc_cont_byte(ch: u32, byte: u8) -> u32 {
        (ch << 6) | (byte & CONT_MASK) as u32
    }

    /// Mask of the value bits of a continuation byte.
    const CONT_MASK: u8 = 0b0011_1111;

    // Decode UTF-8
    let x = *bytes.next()?;
    if x < 128 {
        return Some(x as u32);
    }

    // Multibyte case follows
    // Decode from a byte combination out of: [[[x y] z] w]
    // NOTE: Performance is sensitive to the exact formulation here
    let init = utf8_first_byte(x, 2);
    // SAFETY: `bytes` produces an UTF-8-like string,
    // so the iterator must produce a value here.
    let y = unsafe { *bytes.next().unwrap_unchecked() };
    let mut ch = utf8_acc_cont_byte(init, y);
    if x >= 0xE0 {
        // [[x y z] w] case
        // 5th bit in 0xE0 .. 0xEF is always clear, so `init` is still valid
        // SAFETY: `bytes` produces an UTF-8-like string,
        // so the iterator must produce a value here.
        let z = unsafe { *bytes.next().unwrap_unchecked() };
        let y_z = utf8_acc_cont_byte((y & CONT_MASK) as u32, z);
        ch = init << 12 | y_z;
        if x >= 0xF0 {
            // [x y z w] case
            // use only the lower 3 bits of `init`
            // SAFETY: `bytes` produces an UTF-8-like string,
            // so the iterator must produce a value here.
            let w = unsafe { *bytes.next().unwrap_unchecked() };
            ch = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
        }
    }

    Some(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iter() {
        let mut chars = Chars::from("abcd");
        assert_eq!(chars.head(), Some('a'));
        assert_eq!(chars.next(), Some('a'));
        assert_eq!(chars.head(), Some('b'));
        // assert_eq!(chars.remainer_str(), "cd");
        assert_eq!(chars.next(), Some('b'));
        assert_eq!(chars.head(), Some('c'));
        // assert_eq!(chars.remainer_str(), "d");
    }
}
