use crate::utils::trie::TrieNode;
use shared::{ContentType, Serializable};
use std::collections::HashMap;

type Value = (ContentType, Vec<u8>);

pub struct NikkaDb {
    pub(crate) storage: HashMap<String, Value>,
    pub(crate) trie: TrieNode,
}

impl NikkaDb {
    pub fn add(&mut self, key: String, value: Value) {
        self.trie.insert(&key);
        self.storage.insert(key, value);
    }

    pub fn delete(&mut self, key: &str) {
        self.trie.remove(key);
        self.storage.remove(key);
    }

    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: Serializable,
    {
        let content = self.storage.get(key).cloned();
        match content {
            Some(content_piece) => Some(T::from_bytes(&content_piece.1)),

            None => None,
        }
    }

    pub fn find_regex(&self, regex: &str) -> Vec<String> {
        self.trie.find_regex(regex)
    }
}
