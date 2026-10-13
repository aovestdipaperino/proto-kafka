/// Inputs the spec rejects although Java accepts them. Each variant is
/// documented in docs/FINDINGS.md ("Deliberately stricter cases").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strict {
    /// Length prefix below -1 (Java treats any negative length as null).
    S1NegativeLength,
    /// Invalid UTF-8 in a string (Java replaces with U+FFFD).
    S2InvalidUtf8,
    /// Tagged fields duplicated or not strictly increasing (Java accepts).
    S3TagOrder,
    /// Known tagged field whose value does not exactly fill its declared size (Java ignores size).
    S4TagSize,
    /// Final varint/varlong byte has bits set that Java silently drops.
    S5VarintHighBits,
    // S6 (unknown record batch magic) was retracted: Java's MemoryRecords path rejects
    // magic outside 0..=2, so the spec rejects it with SpecError::Records. See FINDINGS.
    /// Tagged-field count >= 2^31. Java holds it in an `int`, so it is negative and the
    /// read loop runs zero times (the section is accepted as empty).
    S7TagCountOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecError {
    /// Input ended before the value was complete.
    Eof,
    /// Continuation bit set on the last permitted varint byte (Java: illegalVarintException).
    VarintTooLong,
    /// Null length for a field that is not nullable in this version.
    NullNotAllowed(&'static str),
    /// String length above 0x7fff.
    StringTooLong(&'static str),
    /// Array length above the remaining byte count (Java: "Tried to allocate a collection...").
    ArrayTooLong(&'static str),
    /// A known tag that is not valid for this version (Java: "Tag N is not valid for version").
    TagNotValidForVersion(u32),
    UnsupportedVersion(i16),
    /// Encode side: the value tree does not match the schema.
    Shape(String),
    /// Record-set structural error.
    Records(String),
    /// Rejected by the spec although Java accepts. See docs/FINDINGS.md.
    Strict(Strict),
}

pub type Result<T> = core::result::Result<T, SpecError>;
