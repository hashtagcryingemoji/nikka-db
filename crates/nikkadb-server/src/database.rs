use crate::utils::trie::TrieNode;
use std::collections::HashMap;

pub struct NikkaDb {
    pub(crate) storage: HashMap<String, String>,
    pub(crate) trie: TrieNode,
}

impl NikkaDb {
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
