mod common;

use proto_kafka_spec::error::SpecError;
use proto_kafka_spec::interp::{decode, encode};
use proto_kafka_spec::schema::Kind;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn messages_conform_to_java() {
    let mut checked = 0usize;
    let mut strict: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();
    for line in common::lines("messages.jsonl") {
        let schema = common::schema_for(&line);
        let version = line["version"].as_i64().unwrap() as i16;
        let origin = line["origin"].as_str().unwrap();
        let bytes = common::hex(line["bytes"].as_str().unwrap());
        let ctx = format!("{} v{version} {origin} {}", schema.name, line["bytes"]);
        checked += 1;
        match (
            line["ok"].as_bool().unwrap(),
            decode(schema, version, &bytes),
        ) {
            (true, Ok((value, consumed))) => {
                let expected = common::json_struct(schema.root, version, &line["value"]);
                if value != expected {
                    failures.push(format!(
                        "{ctx}: value mismatch\n  spec: {value:?}\n  java: {expected:?}"
                    ));
                } else if consumed as u64 != line["consumed"].as_u64().unwrap() {
                    failures.push(format!(
                        "{ctx}: consumed {consumed}, java {}",
                        line["consumed"]
                    ));
                } else if origin != "mutated" {
                    match encode(schema, version, &value) {
                        Ok(re) if re == bytes => {}
                        other => failures.push(format!("{ctx}: re-encode mismatch: {other:?}")),
                    }
                }
            }
            (true, Err(SpecError::Strict(s))) if origin == "mutated" => {
                *strict.entry(format!("{s:?}")).or_default() += 1;
            }
            (true, Err(e)) => {
                failures.push(format!("{ctx}: Java accepts, spec rejects with {e:?}"))
            }
            (false, Ok(_)) => failures.push(format!(
                "{ctx}: Java rejects ({}), spec accepts",
                line["error"]
            )),
            (false, Err(_)) => {}
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
