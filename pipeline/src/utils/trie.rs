use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

pub struct Trie<T>(pub(super) HashMap<T, TrieNode<T>>);

pub(super) struct TrieNode<T> {
    pub(super) is_end: bool,
    pub(super) children: HashMap<T, TrieNode<T>>,
}

impl<T> Trie<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl<T: Debug> Debug for Trie<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Trie")
            .field("0", &self.0)
            .finish()
    }
}

impl<T: Debug> Debug for TrieNode<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrieNode")
            .field("is_end", &self.is_end)
            .field("children", &self.children)
            .finish()
    }
}

impl<T: Eq + Hash + Clone> Trie<T> {
    pub fn insert(&mut self, key: &[T]) -> bool {
        let mut current_node = self.0.entry(key[0].clone())
            .or_insert(TrieNode {
                is_end: false,
                children: HashMap::new(),
            });
        for part in key.iter().skip(1).cloned() {
            current_node = current_node.children.entry(part).or_insert(TrieNode {
                is_end: false,
                children: HashMap::new(),
            });
        }
        let was_present = current_node.is_end;
        current_node.is_end = true;
        was_present
    }

    pub fn contains(&self, key: &[T]) -> bool {
        let mut current_node = match self.0.get(&key[0]) {
            Some(node) => node,
            None => return false,
        };
        for part in key.iter().skip(1).cloned() {
            match current_node.children.get(&part) {
                Some(node) => current_node = node,
                None => return false,
            }
        }
        current_node.is_end
    }
}
