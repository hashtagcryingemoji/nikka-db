use crate::utils::trie::TrieNode;
use shared::ContentType;
use shared::ContentType::NDeque;
use std::collections::HashMap;

type Value = (ContentType, Vec<u8>);

#[derive(Clone)]
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

    pub fn get(&self, key: &str) -> Option<Value> {
        self.storage.get(key).cloned()
    }

    pub fn find_regex(&self, regex: &str) -> Vec<String> {
        self.trie.find_regex(regex)
    }

    pub fn clear(&mut self) {
        self.storage.clear();
        self.trie = TrieNode::new();
    }

    pub fn pop_first(&mut self, key: &str) -> Option<Value> {
        let deque = self.storage.get(key)?.clone();
        let deque_type = deque.0;
        let mut deque_content = deque.1;

        if deque_content.is_empty() {
            return None;
        }

        match deque_type {
            NDeque(content_type) => {
                let content: Vec<u8> = match *content_type {
                    ContentType::NString => {
                        let len = deque_content[0] as usize;
                        let mut dirty_string: Vec<u8> = deque_content.drain(0..len + 2).collect();
                        dirty_string.remove(0);
                        dirty_string.pop();
                        dirty_string
                    }
                    ContentType::NInt => deque_content.drain(0..1).collect(),
                    _ => unreachable!(),
                };

                self.storage.insert(
                    key.parse().expect("invalid key"),
                    (NDeque(content_type.clone()), deque_content),
                );
                Some((*content_type, content))
            }
            _ => None,
        }
    }

    pub fn pop_last(&mut self, key: &str) -> Option<Value> {
        let deque = self.storage.get(key)?.clone();
        let deque_type = deque.0;
        let mut deque_content = deque.1;
        let deque_len = deque_content.len();

        if deque_len == 0 {
            return None;
        }

        match deque_type {
            NDeque(content_type) => {
                let content = match *content_type {
                    ContentType::NInt => deque_content.drain(deque_len - 1..deque_len).collect(),
                    ContentType::NString => {
                        let len = deque_content[deque_len - 1] as usize;
                        let mut dirty_string: Vec<u8> = deque_content
                            .drain(deque_len - len - 2..deque_len)
                            .collect();
                        dirty_string.remove(0);
                        dirty_string.pop();
                        dirty_string
                    }
                    _ => unreachable!(),
                };
                self.storage.insert(
                    key.parse().expect("invalid key"),
                    (NDeque(content_type.clone()), deque_content),
                );
                Some((*content_type, content))
            }
            _ => None,
        }
    }
}
