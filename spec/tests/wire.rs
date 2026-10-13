use proptest::prelude::*;
use proto_kafka_spec::error::{SpecError, Strict};
use proto_kafka_spec::wire::{LenKind, Mode, Reader, Writer};

#[test]
fn fixed_width_big_endian() {
    let mut r = Reader::new(&[0x01, 0x02, 0xff, 0xff, 0xff, 0xfe]);
    assert_eq!(r.i16().unwrap(), 0x0102);
    assert_eq!(r.i32().unwrap(), -2);
    assert_eq!(r.remaining(), 0);
    assert_eq!(r.u8(), Err(SpecError::Eof));
}

#[test]
fn uvarint_known_encodings() {
    assert_eq!(Reader::new(&[0x00]).uvarint().unwrap(), 0);
    assert_eq!(Reader::new(&[0x7f]).uvarint().unwrap(), 127);
    assert_eq!(Reader::new(&[0x80, 0x01]).uvarint().unwrap(), 128);
    assert_eq!(
        Reader::new(&[0xff, 0xff, 0xff, 0xff, 0x0f])
            .uvarint()
            .unwrap(),
        u32::MAX
    );
    // non-minimal encodings are accepted, as in Java
    assert_eq!(Reader::new(&[0x80, 0x00]).uvarint().unwrap(), 0);
}

#[test]
fn uvarint_fifth_byte_continuation_is_rejected() {
    // docs/FINDINGS.md D1: Java throws, proto-kafka currently misframes.
    let mut r = Reader::new(&[0xff, 0xff, 0xff, 0xff, 0x8f, 0x00]);
    assert_eq!(r.uvarint(), Err(SpecError::VarintTooLong));
}

#[test]
fn uvarint_fifth_byte_high_bits_are_strict() {
    let mut r = Reader::new(&[0xff, 0xff, 0xff, 0xff, 0x1f]);
    assert_eq!(
        r.uvarint(),
        Err(SpecError::Strict(Strict::S5VarintHighBits))
    );
}

#[test]
fn uvarlong_limits() {
    let max = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01];
    assert_eq!(Reader::new(&max).uvarlong().unwrap(), u64::MAX);
    let too_long = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x81, 0x00,
    ];
    assert_eq!(
        Reader::new(&too_long).uvarlong(),
        Err(SpecError::VarintTooLong)
    );
    let high = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x03];
    assert_eq!(
        Reader::new(&high).uvarlong(),
        Err(SpecError::Strict(Strict::S5VarintHighBits))
    );
}

#[test]
fn zigzag() {
    assert_eq!(Reader::new(&[0x01]).varint().unwrap(), -1);
    assert_eq!(Reader::new(&[0x02]).varint().unwrap(), 1);
    assert_eq!(Reader::new(&[0x03]).varlong().unwrap(), -2);
}

#[test]
fn len_prefix_rules() {
    // int16 -1 on nullable => null
    assert_eq!(
        Reader::new(&[0xff, 0xff]).len_prefix(LenKind::Int16, true, "f"),
        Ok(None)
    );
    // int16 -1 on non-nullable => NullNotAllowed
    assert_eq!(
        Reader::new(&[0xff, 0xff]).len_prefix(LenKind::Int16, false, "f"),
        Err(SpecError::NullNotAllowed("f"))
    );
    // int32 -2 on nullable => Strict S1 (Java: null)
    assert_eq!(
        Reader::new(&[0xff, 0xff, 0xff, 0xfe]).len_prefix(LenKind::Int32, true, "f"),
        Err(SpecError::Strict(Strict::S1NegativeLength))
    );
    // compact: 0 => null, 1 => empty, 4 => 3
    assert_eq!(
        Reader::new(&[0x00]).len_prefix(LenKind::Compact, true, "f"),
        Ok(None)
    );
    assert_eq!(
        Reader::new(&[0x01]).len_prefix(LenKind::Compact, true, "f"),
        Ok(Some(0))
    );
    assert_eq!(
        Reader::new(&[0x04]).len_prefix(LenKind::Compact, false, "f"),
        Ok(Some(3))
    );
    // compact raw 0xffffffff => Java int -1, minus 1 => -2 => S1 on nullable
    assert_eq!(
        Reader::new(&[0xff, 0xff, 0xff, 0xff, 0x0f]).len_prefix(LenKind::Compact, true, "f"),
        Err(SpecError::Strict(Strict::S1NegativeLength))
    );
}

#[test]
fn writer_matches_known_encodings() {
    let mut w = Writer::new();
    w.uvarint(300);
    w.varint(-1);
    w.len_prefix(LenKind::Compact, None);
    w.len_prefix(LenKind::Int16, Some(2));
    assert_eq!(w.into_bytes(), vec![0xac, 0x02, 0x01, 0x00, 0x00, 0x02]);
}

proptest! {
    #[test]
    fn uvarint_roundtrip(x: u32) {
        let mut w = Writer::new();
        w.uvarint(x);
        let b = w.into_bytes();
        let mut r = Reader::new(&b);
        prop_assert_eq!(r.uvarint().unwrap(), x);
        prop_assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn varlong_roundtrip(x: i64) {
        let mut w = Writer::new();
        w.varlong(x);
        let b = w.into_bytes();
        prop_assert_eq!(Reader::new(&b).varlong().unwrap(), x);
    }
}

/// Java ByteUtils.readUnsignedVarint: `tmp << 28` drops bits 4..6 of the 5th byte;
/// readUnsignedVarlong: `b << 63` keeps only bit 0 of the 10th byte.
#[test]
fn java_lenient_drops_final_varint_high_bits() {
    let lenient = |b: &'static [u8]| Reader::with_mode(b, Mode::JavaLenient);
    assert_eq!(
        lenient(&[0xff, 0xff, 0xff, 0xff, 0x1f]).uvarint(),
        Ok(u32::MAX)
    );
    assert_eq!(
        lenient(&[0xff, 0xff, 0xff, 0xff, 0x8f, 0x00]).uvarint(),
        Err(SpecError::VarintTooLong)
    );
    assert_eq!(
        lenient(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x03]).uvarlong(),
        Ok(u64::MAX)
    );
}

/// Java MessageDataGenerator: `if (length < 0)` => null for nullable, else throw.
#[test]
fn java_lenient_any_negative_length_is_null() {
    let b = [0xff, 0xff, 0xff, 0xfe];
    assert_eq!(
        Reader::with_mode(&b, Mode::JavaLenient).len_prefix(LenKind::Int32, true, "f"),
        Ok(None)
    );
    assert_eq!(
        Reader::with_mode(&b, Mode::JavaLenient).len_prefix(LenKind::Int32, false, "f"),
        Err(SpecError::NullNotAllowed("f"))
    );
}
