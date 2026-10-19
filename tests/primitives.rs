use proto_kafka::protocol::primitives::*;

#[test]
fn uvarint_decode_known() {
    assert_eq!(read_uvarint(&[0x00]), Ok((0, 1)));
    assert_eq!(read_uvarint(&[0x7f]), Ok((127, 1)));
    assert_eq!(read_uvarint(&[0x80, 0x01]), Ok((128, 2)));
    assert_eq!(
        read_uvarint(&[0xff, 0xff, 0xff, 0xff, 0x0f, 0xaa]),
        Ok((u32::MAX, 5))
    );
    assert_eq!(read_uvarint(&[0x80, 0x00]), Ok((0, 2))); // non-minimal accepted, as Java
    assert_eq!(read_uvarint(&[]), Err(PrimError::Eof));
    assert_eq!(read_uvarint(&[0x80]), Err(PrimError::Eof));
}

#[test]
fn uvarint_fifth_byte_rules() {
    // D1: continuation on the 5th byte is rejected (Java: illegalVarintException)
    assert_eq!(
        read_uvarint(&[0xff, 0xff, 0xff, 0xff, 0x8f, 0x00]),
        Err(PrimError::VarintTooLong)
    );
    // S5: bits Java silently drops are rejected
    assert_eq!(
        read_uvarint(&[0xff, 0xff, 0xff, 0xff, 0x1f]),
        Err(PrimError::VarintOverflow)
    );
}

#[test]
fn uvarlong_tenth_byte_rules() {
    let max = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01];
    assert_eq!(read_uvarlong(&max), Ok((u64::MAX, 10)));
    let cont = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x81, 0x00,
    ];
    assert_eq!(read_uvarlong(&cont), Err(PrimError::VarintTooLong));
    let high = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x03];
    assert_eq!(read_uvarlong(&high), Err(PrimError::VarintOverflow));
}

#[test]
fn uvarint_encode_and_size() {
    let mut out = [0u8; 5];
    let n = write_uvarint(300, &mut out);
    assert_eq!(&out[..n], &[0xac, 0x02]);
    for v in [
        0u32,
        1,
        127,
        128,
        16_383,
        16_384,
        2_097_151,
        2_097_152,
        268_435_455,
        268_435_456,
        u32::MAX,
    ] {
        let n = write_uvarint(v, &mut out);
        assert_eq!(uvarint_size(v), n, "{v}");
        assert_eq!(read_uvarint(&out[..n]), Ok((v, n)), "{v}");
    }
    let mut out = [0u8; 10];
    for v in [0u64, 127, 128, 1 << 35, (1 << 63) - 1, u64::MAX] {
        let n = write_uvarlong(v, &mut out);
        assert_eq!(uvarlong_size(v), n, "{v}");
        assert_eq!(read_uvarlong(&out[..n]), Ok((v, n)), "{v}");
    }
}

#[test]
fn zigzag_known() {
    assert_eq!(zigzag_encode32(0), 0);
    assert_eq!(zigzag_encode32(-1), 1);
    assert_eq!(zigzag_encode32(1), 2);
    assert_eq!(zigzag_encode32(i32::MIN), u32::MAX);
    for v in [0i32, -1, 1, -2, 300, i32::MAX, i32::MIN] {
        assert_eq!(zigzag_decode32(zigzag_encode32(v)), v);
    }
    for v in [0i64, -1, 1, i64::MAX, i64::MIN] {
        assert_eq!(zigzag_decode64(zigzag_encode64(v)), v);
    }
}

#[test]
fn length_rules() {
    assert_eq!(interpret_len(-1), Ok(None));
    assert_eq!(interpret_len(0), Ok(Some(0)));
    assert_eq!(interpret_len(7), Ok(Some(7)));
    assert_eq!(interpret_len(-2), Err(PrimError::NegativeLength(-2)));
    assert_eq!(compact_len(0), -1);
    assert_eq!(compact_len(1), 0);
    assert_eq!(compact_len(u32::MAX), -2);
    assert_eq!(compact_len(0x8000_0000), i32::MAX);
    assert_eq!(check_string_len(0x7fff), Ok(()));
    assert_eq!(
        check_string_len(0x8000),
        Err(PrimError::StringTooLong(0x8000))
    );
    assert_eq!(check_array_len(3, 3), Ok(()));
    assert_eq!(check_array_len(4, 3), Err(PrimError::ArrayTooLong(4, 3)));
}

#[test]
fn struct_marker() {
    assert!(struct_present(1));
    assert!(struct_present(0)); // D4: Java reads the struct when the marker is 0
    assert!(struct_present(2));
    assert!(!struct_present(-1));
    assert!(!struct_present(i8::MIN));
}
