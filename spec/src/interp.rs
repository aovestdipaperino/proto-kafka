//! Schema-driven interpreter: the reference semantics of every Kafka message.
//! Written from the protocol guide and docs/FINDINGS.md, not from proto-kafka.

use crate::error::{Result, SpecError, Strict};
use crate::schema::{DefaultVal, FieldDesc, FieldType, MessageSchema, Prim, StructSchema};
use crate::value::{StructValue, Value};
pub use crate::wire::Mode;
use crate::wire::{LenKind, Reader, Writer};

pub fn is_tagged(f: &FieldDesc, version: i16) -> bool {
    f.tag.is_some() && f.tagged.contains(version)
}

fn field_flexible(f: &FieldDesc, version: i16, msg_flexible: bool) -> bool {
    f.flexible.map_or(msg_flexible, |fv| fv.contains(version))
}

pub fn default_value(f: &FieldDesc, version: i16) -> Value {
    let null = f.default == DefaultVal::Null;
    match f.ty {
        FieldType::Prim(p) => prim_default(p, f.default),
        FieldType::Struct(s) => Value::Struct(if null {
            None
        } else {
            Some(Box::new(default_struct(s, version)))
        }),
        FieldType::PrimArray(_) | FieldType::StructArray(_) => {
            Value::Array(if null { None } else { Some(Vec::new()) })
        }
    }
}

pub fn default_struct(s: &StructSchema, version: i16) -> StructValue {
    StructValue {
        fields: s
            .fields
            .iter()
            .filter(|f| f.versions.contains(version))
            .map(|f| (f.name, default_value(f, version)))
            .collect(),
        unknown_tags: Vec::new(),
    }
}

fn prim_default(p: Prim, d: DefaultVal) -> Value {
    let int = match d {
        DefaultVal::Int(n) => n,
        _ => 0,
    };
    let null = d == DefaultVal::Null;
    match p {
        Prim::Bool => Value::Bool(d == DefaultVal::Bool(true)),
        Prim::Int8 => Value::I8(int as i8),
        Prim::Int16 => Value::I16(int as i16),
        Prim::Uint16 => Value::U16(int as u16),
        Prim::Int32 => Value::I32(int as i32),
        Prim::Int64 => Value::I64(int),
        Prim::Float64 => Value::F64(0f64.to_bits()),
        Prim::Uuid => Value::Uuid([0; 16]),
        Prim::String => Value::Str(match d {
            DefaultVal::Null => None,
            DefaultVal::Str(s) => Some(s.as_bytes().to_vec()),
            _ => Some(Vec::new()),
        }),
        Prim::Bytes => Value::Bytes(if null { None } else { Some(Vec::new()) }),
        // Java: records default is always null.
        Prim::Records => Value::Records(None),
    }
}

/// Strict decode: the reference semantics. Returns the value and the bytes consumed.
pub fn decode(schema: &MessageSchema, version: i16, input: &[u8]) -> Result<(StructValue, usize)> {
    decode_with(schema, version, input, Mode::Strict)
}

/// Decode in the given mode. `Mode::JavaLenient` accepts what Java's generated
/// `*Data(Readable, short)` constructors accept (see `Mode`).
pub fn decode_with(
    schema: &MessageSchema,
    version: i16,
    input: &[u8],
    mode: Mode,
) -> Result<(StructValue, usize)> {
    if !schema.valid.contains(version) {
        return Err(SpecError::UnsupportedVersion(version));
    }
    let mut r = Reader::with_mode(input, mode);
    let v = read_struct(
        &mut r,
        schema.root,
        version,
        schema.flexible.contains(version),
    )?;
    Ok((v, r.pos()))
}

fn read_struct(
    r: &mut Reader<'_>,
    s: &StructSchema,
    version: i16,
    flexible: bool,
) -> Result<StructValue> {
    let mut slots: Vec<Option<Value>> = vec![None; s.fields.len()];
    for (i, f) in s.fields.iter().enumerate() {
        if f.versions.contains(version) && !is_tagged(f, version) {
            slots[i] = Some(read_field(r, f, version, flexible)?);
        }
    }
    let mut unknown_tags = Vec::new();
    if flexible {
        // Java: `int _numTaggedFields = readUnsignedVarint()`; a count >= 2^31 is negative
        // and the loop `for (_i = 0; _i < _numTaggedFields; _i++)` runs zero times.
        let count = match r.uvarint()? {
            n if n > i32::MAX as u32 && r.strict() => {
                return Err(SpecError::Strict(Strict::S7TagCountOverflow))
            }
            n if n > i32::MAX as u32 => 0,
            n => n,
        };
        let mut last: Option<u32> = None;
        for _ in 0..count {
            let tag = r.uvarint()?;
            let size = r.uvarint()? as usize;
            if r.strict() && last.is_some_and(|l| tag <= l) {
                return Err(SpecError::Strict(Strict::S3TagOrder));
            }
            last = Some(tag);
            match s.fields.iter().position(|f| f.tag == Some(tag)) {
                Some(i) => {
                    let f = &s.fields[i];
                    if !(f.versions.contains(version) && is_tagged(f, version)) {
                        return Err(SpecError::TagNotValidForVersion(tag));
                    }
                    slots[i] = Some(if r.strict() {
                        read_tagged_window(r, f, version, flexible, size)?
                    } else {
                        // Java reads a known tag's value from the main stream and ignores
                        // `_size`; a duplicated tag overwrites (last write wins).
                        read_field(r, f, version, flexible)?
                    });
                }
                None => unknown_tags.push((tag, r.take(size)?.to_vec())),
            }
        }
    }
    let fields = s
        .fields
        .iter()
        .zip(slots)
        .filter(|(f, _)| f.versions.contains(version))
        .map(|(f, v)| (f.name, v.unwrap_or_else(|| default_value(f, version))))
        .collect();
    Ok(StructValue {
        fields,
        unknown_tags,
    })
}

/// Strict: a known tag's value must exactly fill its declared `size` (else S4). Java checks
/// lengths against the whole buffer, so running out of the window (Eof, or an array length
/// above the window's remaining bytes) is also S4 rather than a Java-equivalent error.
fn read_tagged_window(
    r: &mut Reader<'_>,
    f: &FieldDesc,
    version: i16,
    flexible: bool,
    size: usize,
) -> Result<Value> {
    let body = r
        .take(size)
        .map_err(|_| SpecError::Strict(Strict::S4TagSize))?;
    let mut sub = Reader::with_mode(body, r.mode());
    let v = read_field(&mut sub, f, version, flexible).map_err(|e| match e {
        SpecError::Eof | SpecError::ArrayTooLong(_) => SpecError::Strict(Strict::S4TagSize),
        e => e,
    })?;
    if sub.remaining() != 0 {
        return Err(SpecError::Strict(Strict::S4TagSize));
    }
    Ok(v)
}

/// A primitive value: the non-recursive leaves of [`Value`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimValue {
    Bool(bool),
    I8(i8),
    I16(i16),
    U16(u16),
    I32(i32),
    I64(i64),
    /// IEEE-754 bits; compared bitwise.
    F64(u64),
    Uuid([u8; 16]),
    Str(Option<Vec<u8>>),
    Bytes(Option<Vec<u8>>),
    Records(Option<Vec<u8>>),
}

impl From<PrimValue> for Value {
    fn from(p: PrimValue) -> Value {
        match p {
            PrimValue::Bool(b) => Value::Bool(b),
            PrimValue::I8(x) => Value::I8(x),
            PrimValue::I16(x) => Value::I16(x),
            PrimValue::U16(x) => Value::U16(x),
            PrimValue::I32(x) => Value::I32(x),
            PrimValue::I64(x) => Value::I64(x),
            PrimValue::F64(x) => Value::F64(x),
            PrimValue::Uuid(u) => Value::Uuid(u),
            PrimValue::Str(s) => Value::Str(s),
            PrimValue::Bytes(b) => Value::Bytes(b),
            PrimValue::Records(b) => Value::Records(b),
        }
    }
}

/// One field as read from the wire, without building a [`Value`] tree. Struct bodies are not
/// read: `Struct` and `StructArray` report what the caller must read next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Prim(PrimValue),
    /// `None` is null.
    PrimArray(Option<Vec<PrimValue>>),
    /// `false`: a nullable struct's null marker was read. `true`: one struct body follows
    /// (after the 1-byte marker of a nullable struct, or immediately for a non-nullable one).
    Struct(bool),
    /// `None` is null; `Some(n)`: `n` struct bodies follow (already checked against the
    /// remaining byte count).
    StructArray(Option<usize>),
}

/// Decodes one non-tagged field of `version` from `input` (the field's flexibility is
/// resolved against `msg_flexible`, as for a field of a message whose flexible flag is
/// `msg_flexible`). Returns the field and the bytes consumed. This is the field-level
/// semantics `decode` uses for every field; it builds no [`Value`] tree.
pub fn decode_field(
    mode: Mode,
    f: &FieldDesc,
    version: i16,
    msg_flexible: bool,
    input: &[u8],
) -> Result<(Field, usize)> {
    let mut r = Reader::with_mode(input, mode);
    let v = field_head(&mut r, f, version, msg_flexible)?;
    Ok((v, r.pos()))
}

fn field_head(r: &mut Reader<'_>, f: &FieldDesc, version: i16, msg_flexible: bool) -> Result<Field> {
    let flex = field_flexible(f, version, msg_flexible);
    let nullable = f.nullable.contains(version);
    match f.ty {
        FieldType::Prim(p) => Ok(Field::Prim(read_prim(r, p, f.name, flex, nullable)?)),
        // Nullable struct: 1-byte marker, negative = null (Java readByte() < 0).
        FieldType::Struct(_) => Ok(Field::Struct(!nullable || r.i8()? >= 0)),
        FieldType::PrimArray(p) => {
            let Some(n) = array_len(r, f.name, flex, nullable)? else {
                return Ok(Field::PrimArray(None));
            };
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(read_prim(r, p, f.name, flex, false)?);
            }
            Ok(Field::PrimArray(Some(items)))
        }
        FieldType::StructArray(_) => Ok(Field::StructArray(array_len(r, f.name, flex, nullable)?)),
    }
}

fn read_field(
    r: &mut Reader<'_>,
    f: &FieldDesc,
    version: i16,
    msg_flexible: bool,
) -> Result<Value> {
    Ok(match (field_head(r, f, version, msg_flexible)?, f.ty) {
        (Field::Prim(p), _) => p.into(),
        (Field::PrimArray(a), _) => {
            Value::Array(a.map(|xs| xs.into_iter().map(Value::from).collect()))
        }
        (Field::Struct(false), _) => Value::Struct(None),
        (Field::Struct(true), FieldType::Struct(s)) => {
            Value::Struct(Some(Box::new(read_struct(r, s, version, msg_flexible)?)))
        }
        (Field::StructArray(None), _) => Value::Array(None),
        (Field::StructArray(Some(n)), FieldType::StructArray(s)) => {
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(Value::Struct(Some(Box::new(read_struct(
                    r,
                    s,
                    version,
                    msg_flexible,
                )?))));
            }
            Value::Array(Some(items))
        }
        (Field::Struct(true) | Field::StructArray(Some(_)), _) => {
            unreachable!("field_head returns Struct/StructArray only for struct field types")
        }
    })
}

/// Array length prefix; a length above the remaining byte count is rejected up front.
fn array_len(
    r: &mut Reader<'_>,
    name: &'static str,
    flex: bool,
    nullable: bool,
) -> Result<Option<usize>> {
    let kind = if flex {
        LenKind::Compact
    } else {
        LenKind::Int32
    };
    let Some(n) = r.len_prefix(kind, nullable, name)? else {
        return Ok(None);
    };
    if n > r.remaining() {
        return Err(SpecError::ArrayTooLong(name));
    }
    Ok(Some(n))
}

fn read_prim(
    r: &mut Reader<'_>,
    p: Prim,
    name: &'static str,
    flex: bool,
    nullable: bool,
) -> Result<PrimValue> {
    Ok(match p {
        Prim::Bool => PrimValue::Bool(r.u8()? != 0),
        Prim::Int8 => PrimValue::I8(r.i8()?),
        Prim::Int16 => PrimValue::I16(r.i16()?),
        Prim::Uint16 => PrimValue::U16(r.u16()?),
        Prim::Int32 => PrimValue::I32(r.i32()?),
        Prim::Int64 => PrimValue::I64(r.i64()?),
        Prim::Float64 => PrimValue::F64(r.u64()?),
        Prim::Uuid => PrimValue::Uuid(r.uuid()?),
        Prim::String => {
            let kind = if flex {
                LenKind::Compact
            } else {
                LenKind::Int16
            };
            PrimValue::Str(match r.len_prefix(kind, nullable, name)? {
                None => None,
                Some(n) if n > 0x7fff => return Err(SpecError::StringTooLong(name)),
                Some(n) => {
                    let b = r.take(n)?;
                    // Java decodes with `new String(arr, UTF_8)` (lossy U+FFFD); JavaLenient
                    // keeps the raw bytes.
                    if r.strict() && core::str::from_utf8(b).is_err() {
                        return Err(SpecError::Strict(Strict::S2InvalidUtf8));
                    }
                    Some(b.to_vec())
                }
            })
        }
        Prim::Bytes | Prim::Records => {
            let kind = if flex {
                LenKind::Compact
            } else {
                LenKind::Int32
            };
            let b = match r.len_prefix(kind, nullable, name)? {
                None => None,
                Some(n) => Some(r.take(n)?.to_vec()),
            };
            if p == Prim::Bytes {
                PrimValue::Bytes(b)
            } else {
                PrimValue::Records(b)
            }
        }
    })
}

pub fn encode(schema: &MessageSchema, version: i16, value: &StructValue) -> Result<Vec<u8>> {
    if !schema.valid.contains(version) {
        return Err(SpecError::UnsupportedVersion(version));
    }
    let mut w = Writer::new();
    write_struct(
        &mut w,
        schema.root,
        version,
        schema.flexible.contains(version),
        value,
    )?;
    Ok(w.into_bytes())
}

fn shape(msg: String) -> SpecError {
    SpecError::Shape(msg)
}

fn write_struct(
    w: &mut Writer,
    s: &StructSchema,
    version: i16,
    flexible: bool,
    v: &StructValue,
) -> Result<()> {
    let present: Vec<&FieldDesc> = s
        .fields
        .iter()
        .filter(|f| f.versions.contains(version))
        .collect();
    if present.len() != v.fields.len()
        || present
            .iter()
            .zip(&v.fields)
            .any(|(f, (n, _))| f.name != *n)
    {
        return Err(shape(format!(
            "{}: fields do not match schema for v{version}",
            s.name
        )));
    }
    for (f, (_, val)) in present.iter().zip(&v.fields) {
        if !is_tagged(f, version) {
            write_field(w, f, version, flexible, val)?;
        }
    }
    if !flexible {
        if !v.unknown_tags.is_empty() {
            return Err(shape(format!(
                "{}: tagged fields in a non-flexible version",
                s.name
            )));
        }
        return Ok(());
    }
    let mut entries: Vec<(u32, Vec<u8>)> = Vec::new();
    for (f, (_, val)) in present.iter().zip(&v.fields) {
        if is_tagged(f, version) && *val != default_value(f, version) {
            let mut sub = Writer::new();
            write_field(&mut sub, f, version, flexible, val)?;
            entries.push((f.tag.expect("tagged field has a tag"), sub.into_bytes()));
        }
    }
    entries.extend(v.unknown_tags.iter().cloned());
    entries.sort_by_key(|(t, _)| *t);
    if entries.windows(2).any(|p| p[0].0 == p[1].0) {
        return Err(shape(format!("{}: duplicate tag", s.name)));
    }
    w.uvarint(entries.len() as u32);
    for (tag, bytes) in entries {
        w.uvarint(tag);
        w.uvarint(bytes.len() as u32);
        w.bytes(&bytes);
    }
    Ok(())
}

fn null_err(name: &str) -> SpecError {
    shape(format!("{name}: null in a non-nullable field"))
}

fn write_field(
    w: &mut Writer,
    f: &FieldDesc,
    version: i16,
    msg_flexible: bool,
    val: &Value,
) -> Result<()> {
    let flex = field_flexible(f, version, msg_flexible);
    let nullable = f.nullable.contains(version);
    match (f.ty, val) {
        (FieldType::Prim(p), v) => write_prim(w, p, f.name, flex, nullable, v),
        (FieldType::Struct(s), Value::Struct(sv)) => match (nullable, sv) {
            (true, None) => {
                w.i8(-1);
                Ok(())
            }
            (true, Some(sv)) => {
                w.i8(1);
                write_struct(w, s, version, msg_flexible, sv)
            }
            (false, Some(sv)) => write_struct(w, s, version, msg_flexible, sv),
            (false, None) => Err(null_err(f.name)),
        },
        (FieldType::PrimArray(p), Value::Array(items)) => {
            write_array(w, f.name, flex, nullable, items.as_deref(), |w, e| {
                write_prim(w, p, f.name, flex, false, e)
            })
        }
        (FieldType::StructArray(s), Value::Array(items)) => write_array(
            w,
            f.name,
            flex,
            nullable,
            items.as_deref(),
            |w, e| match e {
                Value::Struct(Some(sv)) => write_struct(w, s, version, msg_flexible, sv),
                _ => Err(shape(format!("{}: array element is not a struct", f.name))),
            },
        ),
        _ => Err(shape(format!(
            "{}: value does not match field type",
            f.name
        ))),
    }
}

fn write_array(
    w: &mut Writer,
    name: &str,
    flex: bool,
    nullable: bool,
    items: Option<&[Value]>,
    mut elem: impl FnMut(&mut Writer, &Value) -> Result<()>,
) -> Result<()> {
    let kind = if flex {
        LenKind::Compact
    } else {
        LenKind::Int32
    };
    match items {
        None if !nullable => Err(null_err(name)),
        None => {
            w.len_prefix(kind, None);
            Ok(())
        }
        Some(items) => {
            w.len_prefix(kind, Some(items.len()));
            items.iter().try_for_each(|e| elem(w, e))
        }
    }
}

fn write_opt_bytes(
    w: &mut Writer,
    kind: LenKind,
    name: &str,
    nullable: bool,
    b: Option<&[u8]>,
) -> Result<()> {
    match b {
        None if !nullable => Err(null_err(name)),
        None => {
            w.len_prefix(kind, None);
            Ok(())
        }
        Some(b) => {
            w.len_prefix(kind, Some(b.len()));
            w.bytes(b);
            Ok(())
        }
    }
}

fn write_prim(
    w: &mut Writer,
    p: Prim,
    name: &str,
    flex: bool,
    nullable: bool,
    v: &Value,
) -> Result<()> {
    match (p, v) {
        (Prim::Bool, Value::Bool(b)) => w.u8(u8::from(*b)),
        (Prim::Int8, Value::I8(x)) => w.i8(*x),
        (Prim::Int16, Value::I16(x)) => w.i16(*x),
        (Prim::Uint16, Value::U16(x)) => w.u16(*x),
        (Prim::Int32, Value::I32(x)) => w.i32(*x),
        (Prim::Int64, Value::I64(x)) => w.i64(*x),
        (Prim::Float64, Value::F64(bits)) => w.u64(*bits),
        (Prim::Uuid, Value::Uuid(u)) => w.bytes(u),
        (Prim::String, Value::Str(s)) => {
            if s.as_ref().is_some_and(|b| b.len() > 0x7fff) {
                return Err(SpecError::StringTooLong("<encode>"));
            }
            let kind = if flex {
                LenKind::Compact
            } else {
                LenKind::Int16
            };
            write_opt_bytes(w, kind, name, nullable, s.as_deref())?;
        }
        (Prim::Bytes, Value::Bytes(b)) | (Prim::Records, Value::Records(b)) => {
            let kind = if flex {
                LenKind::Compact
            } else {
                LenKind::Int32
            };
            write_opt_bytes(w, kind, name, nullable, b.as_deref())?;
        }
        _ => return Err(shape(format!("{name}: value does not match {p:?}"))),
    }
    Ok(())
}
