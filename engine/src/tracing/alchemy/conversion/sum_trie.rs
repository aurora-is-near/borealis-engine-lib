use std::collections::{BTreeMap, VecDeque};

/// A trie with the property that for all nodes in the trie,
/// `node.value >= node.children.map(|c| c.value).sum()`.
/// The reason for the `>=` is because each node can contribute
/// an amount to the value itself (independent of its children).
#[derive(Debug, Default, Clone)]
pub struct SumTrie<T> {
    root: SumTrieNode<T>,
}

#[derive(Debug, Default, Clone)]
struct SumTrieNode<T> {
    value: T,
    children: BTreeMap<u32, SumTrieNode<T>>,
}

impl<T: Copy> SumTrie<T> {
    pub fn bf_traverse(&self) -> Vec<T> {
        let mut result = Vec::new();
        let mut q = VecDeque::new();
        q.push_back(&self.root);
        while let Some(node) = q.pop_front() {
            result.push(node.value);
            for child in node.children.values() {
                q.push_back(child);
            }
        }
        result
    }
}

impl<T: std::ops::Add<Output = T> + Copy + Default> SumTrie<T> {
    pub fn insert(&mut self, key: &[u32], value: T) {
        self.root.value = self.root.value + value;
        let mut curr_node = &mut self.root;
        for index in key {
            curr_node = curr_node
                .children
                .entry(*index)
                .or_insert_with(Default::default);
            curr_node.value = curr_node.value + value;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sum_trie() {
        // Suppose a tree looks like:
        //      x
        //    / | \
        //   x  x  x
        //  /\     |
        // x  x    x
        //         |
        //         x
        // Then mapping to number of direct children it's:
        //      3
        //    / | \
        //   2  0  1
        //  /\     |
        // 0  0    1
        //         |
        //         0
        // Then the unsummed trie would look like:
        //                 ([], 3)
        //             /      |      \
        //        ([0], 2)  ([1], 0) ([2], 1)
        //        /      \               |
        // ([0, 0], 0)  ([0, 1], 0)  ([2, 0], 1)
        //                               |
        //                         ([2, 0, 0], 0)
        // And so we expect the summed trie to be:
        //                 ([], 7)
        //             /      |      \
        //        ([0], 2)  ([1], 0) ([2], 2)
        //        /      \               |
        // ([0, 0], 0)  ([0, 1], 0)  ([2, 0], 1)
        //                               |
        //                         ([2, 0, 0], 0)
        // Each node in this trie gives the total number of nodes below it.

        let mut trie = super::SumTrie::default();
        let nodes: &[(&[u32], usize)] = &[
            (&[], 3),
            (&[0], 2),
            (&[1], 0),
            (&[2], 1),
            (&[0, 0], 0),
            (&[0, 1], 0),
            (&[2, 0], 1),
            (&[2, 0, 0], 0),
        ];
        for (key, value) in nodes.iter().copied() {
            trie.insert(key, value);
        }
        assert_eq!(trie.bf_traverse(), vec![7, 2, 0, 2, 0, 0, 1, 0]);
    }
}
