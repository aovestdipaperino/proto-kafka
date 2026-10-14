//! The two `unsafe` sites.
//!
//! 1. `StrBytes::as_str` uses `from_utf8_unchecked`. Its safety rests on every constructor
//!    validating UTF-8. The tuple field is private to `protocol::str_bytes`, and the only
//!    `Self(..)` sites are `new` (empty), `from_static_str`/`From<&'static str>` (already `&str`),
//!    `from_string`/`From<String>` (already `String`) and `from_utf8`/`TryFrom<Bytes>`; the
//!    derived `Default` (empty `Bytes`) and `Clone` (copy of a valid value) also preserve it.
//!    Only `from_utf8` takes unchecked bytes. The harnesses prove that `from_utf8` accepts
//!    exactly the valid UTF-8 inputs of up to 4 bytes, and that `as_str` on its result returns
//!    the same bytes. The constructor audit is recorded in FINDINGS.
//! 2. `SegmentedBuf::advance_mut` forwards to `BytesMut::advance_mut` on the last (inline)
//!    segment, the one `chunk_mut` returned. The harnesses verify the forwarded `BytesMut`
//!    contract (write into `chunk_mut`, advance by the number of initialised bytes, read the
//!    same bytes back through the safe API); see `bytesmut_advance_mut` for why the
//!    `SegmentedBuf` wrapper itself is argued by hand rather than model-checked.

use crate::protocol::StrBytes;
use bytes::{Buf, BufMut, Bytes, BytesMut};

/// `from_utf8` accepts exactly the valid UTF-8 inputs of `N` bytes, and `as_str` returns
/// them unchanged. One harness per length (0..=4): with a concrete length CBMC needs no
/// symbolic-size `memcpy`/validation, and the union of the five covers every input of up to
/// 4 bytes (which includes every 1-, 2-, 3- and 4-byte UTF-8 sequence, valid or not).
fn strbytes_from_utf8_exact<const N: usize>() {
    let raw: [u8; N] = kani::any();
    match StrBytes::from_utf8(Bytes::copy_from_slice(&raw)) {
        Ok(s) => {
            assert!(core::str::from_utf8(&raw).is_ok());
            assert_eq!(s.as_str().as_bytes(), &raw[..]);
        }
        Err(_) => {
            assert!(core::str::from_utf8(&raw).is_err());
        }
    }
}

macro_rules! strbytes_harness {
    ($($name:ident = $n:literal),* $(,)?) => {$(
        #[kani::proof]
        #[kani::unwind(6)]
        fn $name() {
            strbytes_from_utf8_exact::<$n>();
        }
    )*};
}

strbytes_harness!(
    strbytes_from_utf8_is_exactly_utf8_len0 = 0,
    strbytes_from_utf8_is_exactly_utf8_len1 = 1,
    strbytes_from_utf8_is_exactly_utf8_len2 = 2,
    strbytes_from_utf8_is_exactly_utf8_len3 = 3,
    strbytes_from_utf8_is_exactly_utf8_len4 = 4,
);

/// Site 2, delegated half: the `BytesMut` contract `SegmentedBuf::advance_mut` forwards to.
/// After `P` symbolic bytes of `put_slice`, writing `K` symbolic bytes into `chunk_mut()` and
/// calling `advance_mut(K)` leaves exactly `P + K` bytes, equal to what was written.
/// `P = 0` exercises `chunk_mut`'s reserve path, `P = 2` the existing-spare-capacity path.
///
/// The same harness on `SegmentedBuf` itself is out of reach for Kani 0.68 / CBMC 6.11: a
/// `BytesMut` held inside the segment `Vec` (heap memory, which CBMC models as bytes) with an
/// int-tagged `data` pointer and an 8192-byte `reserve` does not get through propositional
/// reduction even when every input is concrete. The forwarding half is argued by hand in
/// `docs/FINDINGS.md` ("Phase 2 proofs").
fn bytesmut_advance_mut<const P: usize, const K: usize>() {
    let mut b = BytesMut::new();
    let a: [u8; P] = kani::any();
    b.put_slice(&a);
    let c: [u8; K] = kani::any();
    let chunk = b.chunk_mut();
    assert!(chunk.len() >= K);
    chunk[..K].copy_from_slice(&c);
    // SAFETY: the first `K` bytes of `chunk_mut()` were initialised just above.
    unsafe { b.advance_mut(K) };
    assert_eq!(b.remaining(), P + K);
    assert_eq!(&b[..P], &a);
    assert_eq!(&b[P..], &c);
}

macro_rules! bytesmut_harness {
    ($($name:ident = ($p:literal, $k:literal)),* $(,)?) => {$(
        #[kani::proof]
        #[kani::unwind(6)]
        fn $name() {
            bytesmut_advance_mut::<$p, $k>();
        }
    )*};
}

bytesmut_harness!(
    bytesmut_advance_mut_keeps_written_bytes_p0_k0 = (0, 0),
    bytesmut_advance_mut_keeps_written_bytes_p0_k1 = (0, 1),
    bytesmut_advance_mut_keeps_written_bytes_p0_k2 = (0, 2),
    bytesmut_advance_mut_keeps_written_bytes_p0_k3 = (0, 3),
    bytesmut_advance_mut_keeps_written_bytes_p2_k0 = (2, 0),
    bytesmut_advance_mut_keeps_written_bytes_p2_k1 = (2, 1),
    bytesmut_advance_mut_keeps_written_bytes_p2_k2 = (2, 2),
    bytesmut_advance_mut_keeps_written_bytes_p2_k3 = (2, 3),
);
