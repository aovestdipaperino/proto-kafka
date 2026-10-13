//! Schema-driven interpreter: the reference semantics of every Kafka message.
//! Written from the protocol guide and docs/FINDINGS.md, not from proto-kafka.

use crate::error::{Result, SpecError, Strict};
use crate::schema::{DefaultVal, FieldDesc, FieldType, MessageSchema, Prim, StructSchema};
use crate::value::{StructValue, Value};
use crate::wire::{LenKind, Reader};

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

pub fn decode(schema: &MessageSchema, version: i16, input: &[u8]) -> Result<(StructValue, usize)> {
    if !schema.valid.contains(version) {
        return Err(SpecError::UnsupportedVersion(version));
    }
    let mut r = Reader::new(input);
    let v = read_struct(&mut r, schema.root, version, schema.flexible.contains(version))?;
    Ok((v, r.pos()))
}

fn read_struct(r: &mut Reader<'_>, s: &StructSchema, version: i16, flexible: bool) -> Result<StructValue> {
    let mut slots: Vec<Option<Value>> = vec![None; s.fields.len()];
    for (i, f) in s.fields.iter().enumerate() {
        if f.versions.contains(version) && !is_tagged(f, version) {
            slots[i] = Some(read_field(r, f, version, flexible)?);
        }
    }
    let mut unknown_tags = Vec::new();
    if flexible {
        let count = r.uvarint()?;
        let mut last: Option<u32> = None;
        for _ in 0..count {
            let tag = r.uvarint()?;
            let size = r.uvarint()? as usize;
            if last.is_some_and(|l| tag <= l) {
                return Err(SpecError::Strict(Strict::S3TagOrder));
            }
            last = Some(tag);
            match s.fields.iter().position(|f| f.tag == Some(tag)) {
                Some(i) => {
                    let f = &s.fields[i];
                    if !(f.versions.contains(version) && is_tagged(f, version)) {
                        return Err(SpecError::TagNotValidForVersion(tag));
                    }
                    // Java ignores `size` for known tags; the spec requires the value to fill it exactly.
                    let body = r.take(size).map_err(|_| SpecError::Strict(Strict::S4TagSize))?;
                    let mut sub = Reader::new(body);
                    let v = read_field(&mut sub, f, version, flexible).map_err(|e| match e {
                        SpecError::Eof => SpecError::Strict(Strict::S4TagSize),
                        e => e,
                    })?;
                    if sub.remaining() != 0 {
                        return Err(SpecError::Strict(Strict::S4TagSize));
                    }
                    slots[i] = Some(v);
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
    Ok(StructValue { fields, unknown_tags })
}

fn read_field(r: &mut Reader<'_>, f: &FieldDesc, version: i16, msg_flexible: bool) -> Result<Value> {
    let flex = field_flexible(f, version, msg_flexible);
    let nullable = f.nullable.contains(version);
    match f.ty {
        FieldType::Prim(p) => read_prim(r, p, f.name, flex, nullable),
        FieldType::Struct(s) => {
            // Nullable struct: 1-byte marker, negative = null (Java readByte() < 0).
            if nullable && r.i8()? < 0 {
                return Ok(Value::Struct(None));
            }
            Ok(Value::Struct(Some(Box::new(read_struct(r, s, version, msg_flexible)?))))
        }
        FieldType::PrimArray(p) => read_array(r, f.name, flex, nullable, |r| read_prim(r, p, f.name, flex, false)),
        FieldType::StructArray(s) => read_array(r, f.name, flex, nullable, |r| {
            Ok(Value::Struct(Some(Box::new(read_struct(r, s, version, msg_flexible)?))))
        }),
    }
}

fn read_array<'a>(
    r: &mut Reader<'a>,
    name: &'static str,
    flex: bool,
    nullable: bool,
    mut elem: impl FnMut(&mut Reader<'a>) -> Result<Value>,
) -> Result<Value> {
    let kind = if flex { LenKind::Compact } else { LenKind::Int32 };
    let Some(n) = r.len_prefix(kind, nullable, name)? else {
        return Ok(Value::Array(None));
    };
    if n > r.remaining() {
        return Err(SpecError::ArrayTooLong(name));
    }
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        items.push(elem(r)?);
    }
    Ok(Value::Array(Some(items)))
}

fn read_prim(r: &mut Reader<'_>, p: Prim, name: &'static str, flex: bool, nullable: bool) -> Result<Value> {
    Ok(match p {
        Prim::Bool => Value::Bool(r.u8()? != 0),
        Prim::Int8 => Value::I8(r.i8()?),
        Prim::Int16 => Value::I16(r.i16()?),
        Prim::Uint16 => Value::U16(r.u16()?),
        Prim::Int32 => Value::I32(r.i32()?),
        Prim::Int64 => Value::I64(r.i64()?),
        Prim::Float64 => Value::F64(r.u64()?),
        Prim::Uuid => Value::Uuid(r.uuid()?),
        Prim::String => {
            let kind = if flex { LenKind::Compact } else { LenKind::Int16 };
            Value::Str(match r.len_prefix(kind, nullable, name)? {
                None => None,
                Some(n) if n > 0x7fff => return Err(SpecError::StringTooLong(name)),
                Some(n) => {
                    let b = r.take(n)?;
                    if core::str::from_utf8(b).is_err() {
                        return Err(SpecError::Strict(Strict::S2InvalidUtf8));
                    }
                    Some(b.to_vec())
                }
            })
        }
        Prim::Bytes | Prim::Records => {
            let kind = if flex { LenKind::Compact } else { LenKind::Int32 };
            let b = match r.len_prefix(kind, nullable, name)? {
                None => None,
                Some(n) => Some(r.take(n)?.to_vec()),
            };
            if p == Prim::Bytes {
                Value::Bytes(b)
            } else {
                Value::Records(b)
            }
        }
    })
}
