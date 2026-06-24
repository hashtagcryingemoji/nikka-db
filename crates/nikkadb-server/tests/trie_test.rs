use nikkadb_server::utils::trie::TrieNode;

#[test]
fn trie_insert_test() {
    let mut t = TrieNode::new();

    t.insert("six");

    assert!(t.find("six"));
    assert!(!t.find("si"));
    assert!(!t.find("seven"));
}

#[test]
fn trie_remove_test() {
    let mut t = TrieNode::new();

    t.insert("six");
    t.remove("six");

    assert!(!t.find("six"));
}

#[test]
fn regex_test() {
    let mut t = TrieNode::new();

    let regex = "g*pherism";

    t.insert("gadsfpherism");
    t.insert("gosdfpherism");
    t.insert("gopherism");

    t.remove("gopherism");

    let v = t.find_regex(regex);
    assert_eq!(
        v,
        vec!["gadsfpherism".to_string(), "gosdfpherism".to_string()]
    )
}
