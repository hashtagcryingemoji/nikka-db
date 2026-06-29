use shared::Serializable;
use std::collections::HashMap;

#[test]
fn hash_map_serialization_test() {
    let mut hm = HashMap::new();

    hm.insert("alice".to_string(), "bob".to_string());
    hm.insert("charlie".to_string(), "duke".to_string());

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
