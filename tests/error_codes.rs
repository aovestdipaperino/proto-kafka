use kafka_protocol::error::{ParseResponseErrorCode, ResponseError};

#[test]
fn error_code_zero_is_ok() {
    assert!(0i16.is_ok());
    assert!(!0i16.is_err());
    assert_eq!(0i16.ok(), Some(()));
    assert_eq!(0i16.err(), None);
}

#[test]
fn known_error_code_parses() {
    let err = (-1i16).err();
    assert!(err.is_some());
    assert_eq!(err.unwrap(), ResponseError::UnknownServerError);
}

#[test]
fn unknown_error_code_wraps() {
    let err = 9999i16.err();
    assert!(err.is_some());
    assert_eq!(err.unwrap(), ResponseError::Unknown(9999));
}

#[test]
fn error_code_roundtrip() {
    let original = ResponseError::OffsetOutOfRange;
    assert_eq!(original.code(), 1);
    let parsed = 1i16.err().unwrap();
    assert_eq!(parsed, original);
}

#[test]
fn retriable_errors() {
    assert!(ResponseError::CorruptMessage.is_retriable());
    assert!(ResponseError::NotLeaderOrFollower.is_retriable());
    assert!(ResponseError::RequestTimedOut.is_retriable());
    assert!(ResponseError::UnknownTopicId.is_retriable());
}

#[test]
fn non_retriable_errors() {
    assert!(!ResponseError::UnknownServerError.is_retriable());
    assert!(!ResponseError::OffsetOutOfRange.is_retriable());
    assert!(!ResponseError::InvalidRequest.is_retriable());
    assert!(!ResponseError::Unknown(999).is_retriable());
}

#[test]
fn display_known_error() {
    let err = ResponseError::OffsetOutOfRange;
    let s = format!("{err}");
    assert_eq!(s, "OffsetOutOfRange");
}

#[test]
fn display_unknown_error() {
    let err = ResponseError::Unknown(12345);
    let s = format!("{err}");
    assert!(s.contains("12345"));
}

#[test]
fn error_is_std_error() {
    let err = ResponseError::InvalidRequest;
    let _: &dyn std::error::Error = &err;
}
