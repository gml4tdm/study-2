use std::hash::Hash;
use crate::utils::trie::{Trie, TrieNode};

pub enum Tree<T> {
    Node{payload: T, children: Vec<Tree<T>>},
    Leaf{payload: T}
}

impl<T: Eq + Hash + Clone> From<Trie<T>> for Vec<Tree<Vec<T>>> {
    fn from(value: Trie<T>) -> Self {
        let mut result = Vec::new();
        let mut stack = Vec::new();
        for (key, node) in value.0 {
            stack.push(key);
            result.push(build_tree_recursive(node, &mut stack));
            stack.pop();
        }
        result 
    }
}


fn build_tree_recursive<T>(node: TrieNode<T>,
                           stack: &mut Vec<T>) -> Tree<Vec<T>> 
where 
    T: Clone 
{
    if node.is_end {
        Tree::Leaf{payload: stack.clone()}
    } else {
        let mut children = Vec::new();
        for (key, child) in node.children {
            stack.push(key);
            children.push(build_tree_recursive(child, stack));
            stack.pop();
        }
        Tree::Node {
            payload: stack.clone(),
            children 
        }
    }
}
