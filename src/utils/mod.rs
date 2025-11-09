pub mod time;

use bytes::Bytes;
use std::fmt;

pub struct TruncateDisplay<'a> {
    value: &'a str,
    limit: usize,
}

impl<'a> TruncateDisplay<'a> {
    pub fn new(value: &'a str, limit: usize) -> Self {
        Self { value, limit }
    }
}

impl<'a> fmt::Display for TruncateDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.value.len() > self.limit {
            write!(f, "{}...", &self.value[..self.limit])
        } else {
            write!(f, "{}", self.value)
        }
    }
}

pub struct BytesTruncated<'a> {
    bytes: &'a Bytes,
    limit: usize,
}

impl<'a> BytesTruncated<'a> {
    pub fn new(bytes: &'a Bytes, limit: usize) -> Self {
        Self { bytes, limit }
    }
}

impl<'a> fmt::Display for BytesTruncated<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let truncated = self.bytes.len() > self.limit;
        let len_to_print = if truncated {
            self.limit
        } else {
            self.bytes.len()
        };

        for (i, byte) in self.bytes.iter().take(len_to_print).enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{:02X}", byte)?;
        }

        if truncated {
            if len_to_print > 0 {
                write!(f, " ")?;
            }
            write!(f, "...")?;
        }

        Ok(())
    }
}
