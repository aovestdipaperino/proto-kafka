use proto_kafka_spec::interp::decode;
use proto_kafka_spec::schema::{find, find_by_name, FieldType, Kind, Prim, Versions};
use proto_kafka_spec::value::Value;

#[test]
fn find_coordinator_request_table() {
    let m = find(10, Kind::Request).expect("FindCoordinatorRequest");
    assert_eq!(m.name, "FindCoordinatorRequest");
    assert_eq!(m.valid, Versions::range(0, 6));
    assert_eq!(m.flexible, Versions::since(3));
    let names: Vec<_> = m.root.fields.iter().map(|f| f.name).collect();
    assert_eq!(names, ["Key", "KeyType", "CoordinatorKeys"]);
    assert!(matches!(
        m.root.fields[2].ty,
        FieldType::PrimArray(Prim::String)
    ));
}

#[test]
fn every_api_key_has_request_and_response() {
    let reqs = proto_kafka_spec::generated_schemas::ALL
        .iter()
        .filter(|m| m.kind == Kind::Request)
        .count();
    let resps = proto_kafka_spec::generated_schemas::ALL
        .iter()
        .filter(|m| m.kind == Kind::Response)
        .count();
    assert_eq!(reqs, resps);
    assert!(reqs >= 87, "found {reqs} requests");
}

#[test]
fn request_header_v2_client_id_is_not_compact() {
    let h = find_by_name("RequestHeader").unwrap();
    // api_key=3, api_version=12, correlation_id=1, ClientId int16 len 2 "ab", empty tagged section
    let bytes = [0, 3, 0, 12, 0, 0, 0, 1, 0, 2, b'a', b'b', 0x00];
    let (v, n) = decode(h, 2, &bytes).unwrap();
    assert_eq!(n, bytes.len());
    assert_eq!(v.get("ClientId"), Some(&Value::Str(Some(b"ab".to_vec()))));
}
