use crate::utils::trie::TrieNode;
use std::collections::HashMap;

pub struct NikkaDb {
    storage: HashMap<String, String>,
    trie: TrieNode,
}

impl NikkaDb {
    pub fn new() -> Self {
        NikkaDb {
            storage: HashMap::new(),
            trie: TrieNode::new(),
        }
    }

    pub fn add(&mut self, key: String, value: String) {
        self.trie.insert(&key);
        self.storage.insert(key, value);
        //println!("{:?}", self.storage);
    }

    pub fn delete(&mut self, key: &str) {
        self.trie.remove(key);
        self.storage.remove(key);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.storage.get(key).cloned()
    }

    pub fn find_regex(&self, regex: &str) -> Vec<String> {
        self.trie.find_regex(regex)
    }
}
