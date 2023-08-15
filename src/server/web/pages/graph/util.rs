use std::collections::{HashMap, HashSet};

/// Sort commits topologically such that parents come before their children.
///
/// Assumes that `parent_child_pairs` contains no duplicates and is in the
/// desired order (see below for more info on the order).
///
/// The algorithm used is a version of [Kahn's algorithm][0] that starts at the
/// nodes with no parents. It uses a stack for the set of parentless nodes,
/// meaning the resulting commit order is depth-first-y, not breadth-first-y.
/// For example, this commit graph (where children are ordered top to bottom)
/// results in the order `A, B, C, D, E, F` and not an interleaved order like
/// `A, B, D, C, E, F` (which a queue would produce):
///
/// ```text
/// A - B - C
///  \       \
///   D - E - F
/// ```
///
/// When a node is visited and added to the list of sorted nodes, it is removed
/// as parent from all its children. Those who had no other parents are added to
/// the stack in reverse order. In the final list, the children appear in the
/// order they appeared in the parent child pairs, if possible. This means that
/// the order of the commits and of the pairs matters and should probably be
/// deterministic.
///
/// [0]: https://en.wikipedia.org/wiki/Topological_sorting#Kahn's_algorithm
pub fn sort_topologically(
    commits: &[String],
    parent_child_pairs: &[(String, String)],
) -> Vec<String> {
    // These maps have entries for each commit hash we might want to inspect, so
    // we know `.get()`, `.get_mut()` and `.remove()` must always succeed.
    let mut parent_child_map = commits
        .iter()
        .map(|hash| (hash.clone(), Vec::<String>::new()))
        .collect::<HashMap<_, _>>();
    let mut child_parent_map = commits
        .iter()
        .map(|hash| (hash.clone(), HashSet::<String>::new()))
        .collect::<HashMap<_, _>>();
    for (parent, child) in parent_child_pairs {
        parent_child_map
            .get_mut(parent)
            .unwrap()
            .push(child.clone());
        child_parent_map
            .get_mut(child)
            .unwrap()
            .insert(parent.clone());
    }

    // Initialize parentless stack using commit list, in reverse order so that
    // the order is right when popping.
    let mut parentless = Vec::<String>::new();
    for commit in commits.iter().rev() {
        if child_parent_map[commit].is_empty() {
            // A (quadratic-time) linear scan here is OK since the number of
            // parentless commits is usually fairly small.
            if !parentless.contains(commit) {
                parentless.push(commit.clone());
            }
        }
    }

    let mut sorted = Vec::<String>::new();
    while let Some(hash) = parentless.pop() {
        // Inspect children in reverse order so that the order is right when
        // popping off the parentless stack.
        for child in parent_child_map.remove(&hash).unwrap().into_iter().rev() {
            let child_parents = child_parent_map.get_mut(&child).unwrap();
            child_parents.remove(&hash);
            if child_parents.is_empty() {
                parentless.push(child);
            }
        }

        sorted.push(hash);
    }

    assert!(parent_child_map.is_empty());
    assert!(child_parent_map.values().all(|v| v.is_empty()));
    assert!(parentless.is_empty());
    assert_eq!(commits.len(), sorted.len());
    sorted
}
