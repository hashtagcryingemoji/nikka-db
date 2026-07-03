use shared::protocol::Response::{ContentResponse, Error};
use shared::protocol::{Request, Response};
use shared::Action::CREATE;
use shared::ContentType::{KeyValue, NString};
use shared::Serializable;
use std::collections::HashMap;

#[test]
fn hash_map_serialization_test() {
    let mut hm = HashMap::new();

    let bob_bytes = "bob".to_string().as_bytes().to_vec();
    let duke_bytes = "duke".to_string().as_bytes().to_vec();

    hm.insert("alice".to_string(), (NString, bob_bytes));
    hm.insert("charlie".to_string(), (NString, duke_bytes));

    let hash_map = hm.clone();

    let bytes = hm.as_bytes();

    let hm = HashMap::from_bytes(&bytes);

    assert_eq!(hm, hash_map)
}

#[test]
fn empty_map_serialization_test() {
    let hm = HashMap::new();
    let hash_map = hm.clone();

    let bytes = hm.as_bytes();
    let hm = HashMap::from_bytes(&bytes);

    assert_eq!(hm, hash_map)
}

#[test]
fn string_vec_deser_test() {
    let v = vec![
        "alice".to_string(),
        "bob".to_string(),
        "charlie".to_string(),
    ];

    let dv = Vec::from_bytes(&v.as_bytes());

    assert_eq!(v, dv);
}

#[test]
fn response_serde_test() {
    let error_response = Error("error is occurred because of claude code in prod".to_string());
    let deserialized_error_response = Response::from_bytes(&error_response.as_bytes());

    let content = "database value".to_string();
    let content_response = ContentResponse(NString, content.as_bytes().to_vec());
    let deserialized_content_response = Response::from_bytes(&content_response.as_bytes());

    assert_eq!(error_response, deserialized_error_response);
    assert_eq!(content_response, deserialized_content_response);
}

#[test]
fn request_serde_test() {
    let request = Request {
        action: CREATE,
        content_type: NString,
        args: vec![12, 13, 14],
    };
    let deserialized_request = Request::from_bytes(&request.as_bytes());

    assert_eq!(request, deserialized_request);
}

#[test]
fn test_nested_types_serde() {
    let request = Request {
        action: CREATE,
        content_type: KeyValue(Box::new(NString)),
        args: vec![12, 13, 14],
    };
    let deserialized_request = Request::from_bytes(&request.as_bytes());

    assert_eq!(request, deserialized_request);
}
