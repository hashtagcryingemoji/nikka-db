type Pair<'a> = (String, &'a TrieNode);

#[derive(Debug, Clone)]
pub struct TrieNode {
    children: Vec<TrieNode>,
    char: char,
    is_terminal: bool,
}

impl TrieNode {
    #[must_use]
    pub fn new() -> Self {
        TrieNode {
            children: Vec::new(),
            char: '\0',
            is_terminal: false,
        }
    }

    pub fn insert(&mut self, word: &str) {
        let chars: Vec<char> = word.chars().collect();
        let mut cn = self;

        for i in &chars {
            let found_node = cn.children.iter().position(|j| &j.char == i);

            if let Some(index) = found_node {
                cn = &mut cn.children[index];
            } else {
                cn.children.push(TrieNode {
                    children: Vec::new(),
                    char: *i,
                    is_terminal: false,
                });

                cn = cn.children.last_mut().expect("logic error");
            }
        }

        cn.is_terminal = true;
    }

    pub fn find(&self, word: &str) -> bool {
        let chars: Vec<char> = word.chars().collect();
        let mut cn = self;

        for i in &chars {
            let found_node = cn.children.iter().position(|j| &j.char == i);

            match found_node {
                Some(index) => cn = cn.children.get(index).unwrap(),
                None => return false,
            }
        }

        cn.is_terminal
    }

    #[must_use]
    pub fn find_regex(&self, regex_pattern: &str) -> Vec<String> {
        let mut v = Vec::new();

        //end case
        if regex_pattern.is_empty() {
            return vec![];
        }

        //single char
        if regex_pattern.len() == 1 {
            if regex_pattern == "%" {
                for child in &self.children {
                    if child.is_terminal {
                        v.push(self.char.to_string() + &child.char.to_string());
                    }
                }
                return v;
            }
            if regex_pattern == "*" {
                let mut v = Vec::new();
                for word in self.get_all() {
                    v.push(self.char.to_string() + &word);
                }
                return v;
            }

            if self.find(regex_pattern) {
                return vec![self.char.to_string() + regex_pattern];
            }
        }

        if regex_pattern[..1] == *"*" {
            for child in &self.children {
                let child_regex = child.get_all_before_char(&regex_pattern[1..2]);

                for pair in child_regex {
                    for word in pair.1.find_regex(&regex_pattern[2..]) {
                        if self.char == '\0' {
                            let new_word = child.char.to_string() + &pair.0 + &word;
                            v.push(new_word);
                        } else {
                            let new_word =
                                self.char.to_string() + &child.char.to_string() + &pair.0 + &word;
                            v.push(new_word);
                        }
                    }
                }
            }

            return v;
        }

        if regex_pattern[..1] == *"%" {
            for child in &self.children {
                let child_regex = child.find_regex(&regex_pattern[1..]);
                for word in child_regex {
                    if self.char == '\0' {
                        v.push(word);
                    } else {
                        let new_word = self.char.to_string() + &word;
                        v.push(new_word);
                    }
                }
            }

            return v;
        }

        let found_node = self
            .children
            .iter()
            .position(|j| j.char.to_string() == regex_pattern[..1]);

        match found_node {
            Some(index) => {
                let chosen_child = self.children.get(index).expect("logic error");
                for word in chosen_child.find_regex(&regex_pattern[1..]) {
                    if self.char == '\0' {
                        v.push(word);
                    } else {
                        let new_word = self.char.to_string() + &word;
                        v.push(new_word);
                    }
                }
                v
            }
            None => vec![],
        }
    }

    pub fn remove(&mut self, word: &str) {
        let chars: Vec<char> = word.chars().collect();
        let mut cn = self;

        for i in &chars {
            let found_node = cn.children.iter().position(|j| &j.char == i);

            match found_node {
                Some(index) => {
                    cn = &mut cn.children[index];
                }
                None => {
                    return;
                }
            }
        }

        cn.is_terminal = false;
    }

    fn get_all(&self) -> Vec<String> {
        let mut v = Vec::new();

        for child in &self.children {
            if child.is_terminal {
                v.push(child.char.to_string());
            }

            for word in &child.get_all() {
                v.push(child.char.to_string() + word);
            }
        }

        v
    }

    pub fn get_all_before_char(&'_ self, chosen_char: &str) -> Vec<Pair<'_>> {
        let mut v = Vec::new();

        for child in &self.children {
            if child.char.to_string() == chosen_char {
                v.push((String::new(), child));
            }

            for word in &child.get_all_before_char(chosen_char) {
                v.push((child.char.to_string() + &word.0, word.1));
            }
        }

        v
    }
}

impl Default for TrieNode {
    fn default() -> Self {
        Self::new()
    }
}
