mod common;

use proto_kafka_spec::error::SpecError;
use proto_kafka_spec::interp::{decode, decode_with, encode, Mode};
use proto_kafka_spec::schema::Kind;
use std::collections::{BTreeMap, BTreeSet};

/// Strict rejections of Java-accepted vectors, by variant. Asserted exactly: a change
/// means the spec's strictness changed and docs/FINDINGS.md must be updated with it.
const EXPECTED_STRICT: &[(&str, usize)] = &[
    ("S1NegativeLength", 45),
    ("S2InvalidUtf8", 439),
    ("S3TagOrder", 1),
    ("S4TagSize", 6),
];

/// For every vector:
/// - Java accepts: the `JavaLenient` decode equals Java's value (strings compared after
///   lossy UTF-8 decoding, as Java does) and consumed length; the strict decode either
///   equals the lenient one or is `Err(Strict(_))` (mutated vectors only); valid vectors
///   re-encode byte-for-byte.
/// - Java rejects: both strict and lenient decodes reject.
#[test]
fn messages_conform_to_java() {
    let mut checked = 0usize;
    let mut strict: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();
    for line in common::lines("messages.jsonl") {
        let schema = common::schema_for(&line);
        let version = line["version"].as_i64().unwrap() as i16;
        let origin = line["origin"].as_str().unwrap();
        let mutated = common::is_mutated(origin);
        let bytes = common::hex(line["bytes"].as_str().unwrap());
        let ctx = format!("{} v{version} {origin} {}", schema.name, line["bytes"]);
        checked += 1;
        let strict_res = decode(schema, version, &bytes);
        let lenient_res = decode_with(schema, version, &bytes, Mode::JavaLenient);
        if !line["ok"].as_bool().unwrap() {
            if strict_res.is_ok() || lenient_res.is_ok() {
                failures.push(format!(
                    "{ctx}: Java rejects ({}), spec accepts (strict ok: {}, lenient ok: {})",
                    line["error"],
                    strict_res.is_ok(),
                    lenient_res.is_ok()
                ));
            }
            continue;
        }
        let (lenient, lenient_consumed) = match lenient_res {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{ctx}: Java accepts, lenient rejects with {e:?}"));
                continue;
            }
        };
        let expected = common::json_struct(schema.root, version, &line["value"]);
        let lossy = common::lossy_struct(&lenient);
        if lossy != expected {
            failures.push(format!(
                "{ctx}: lenient value mismatch\n  spec: {lossy:?}\n  java: {expected:?}"
            ));
            continue;
        }
        if lenient_consumed as u64 != line["consumed"].as_u64().unwrap() {
            failures.push(format!(
                "{ctx}: lenient consumed {lenient_consumed}, java {}",
                line["consumed"]
            ));
            continue;
        }
        match strict_res {
            Ok((value, consumed)) => {
                if value != lenient || consumed != lenient_consumed {
                    failures.push(format!("{ctx}: strict decode differs from lenient"));
                } else if !mutated {
                    match encode(schema, version, &value) {
                        Ok(re) if re == bytes => {}
                        other => failures.push(format!("{ctx}: re-encode mismatch: {other:?}")),
                    }
                }
            }
            Err(SpecError::Strict(s)) if mutated => {
                *strict.entry(format!("{s:?}")).or_default() += 1;
            }
            Err(e) => failures.push(format!("{ctx}: Java accepts, strict rejects with {e:?}")),
        }
    }
    eprintln!(
        "checked {checked} message vectors; strict rejections where Java accepts: {strict:?}"
    );
    assert!(
        failures.is_empty(),
        "{} failures; first 20:\n{}",
        failures.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    let expected: BTreeMap<String, usize> = EXPECTED_STRICT
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();
    assert_eq!(strict, expected, "strict histogram drifted");
}

#[test]
fn golden_covers_every_message_version() {
    let mut seen: BTreeSet<(String, i16)> = BTreeSet::new();
    for line in common::lines("messages.jsonl") {
        if line["origin"] != "mutated" && line["ok"] == true {
            seen.insert((
                common::schema_for(&line).name.to_string(),
                line["version"].as_i64().unwrap() as i16,
            ));
        }
    }
    let mut missing = Vec::new();
    for m in proto_kafka_spec::generated_schemas::ALL
        .iter()
        .filter(|m| matches!(m.kind, Kind::Request | Kind::Response))
    {
        for v in m.valid.min..=m.valid.max {
            if !seen.contains(&(m.name.to_string(), v)) {
                missing.push(format!("{} v{v}", m.name));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "no valid golden vector for: {missing:?}"
    );
}
