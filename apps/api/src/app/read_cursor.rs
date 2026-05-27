#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventSourceCursor {
    pub unix_micros: u64,
    pub tie_breaker: Option<Box<str>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowSourceCursor {
    pub unix_micros: u64,
    pub tie_breaker: Option<Box<str>>,
}
