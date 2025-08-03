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
            // 如果超长，截断并附上 "..."
            write!(f, "{}...", &self.value[..self.limit])
        } else {
            // 否则，正常打印
            write!(f, "{}", self.value)
        }
    }
}
