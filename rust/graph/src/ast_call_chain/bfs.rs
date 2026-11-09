//! BFS traversal over the call-graph adjacency of the query index.
//!
//! Every adjacency list is sorted by slug (KGRAPH-077), and every BFS level
//! is sorted before it is emitted, so the output depends only on the graph.
//!
//! - `find_paths`: the call chains between two functions (KGRAPH-060)
//! - `find_all_reachable`: every function reachable from a source, with its
//!   depth (KGRAPH-061)

use std::collections::{BTreeSet, HashMap, HashSet};

/// Slug → the sorted slugs it has an edge to.
pub type Adjacency = HashMap<String, Vec<String>>;

/// The call chains from `from_slug` to `to_slug`, and whether more than
/// `max_chains` exist.
pub struct Chains {
    /// The chains, shortest first, then in the order of their slug sequences.
    pub chains: Vec<Vec<String>>,
    /// Whether chains beyond `max_chains` were left out.
    pub truncated: bool,
}

/// The chains from `from_slug` to `to_slug` within `max_depth` calls: every
/// path whose every function before the target is at its shortest call
/// distance from the source, that is a shortest path to a direct caller of
/// the target followed by the call to the target.
///
/// Distances and parents come from one BFS that does not go through the
/// target; the chains to each caller of the target are enumerated in slug
/// order over that shortest-path DAG, so memory stays bounded by the
/// number of chains returned.
pub fn find_paths(
    forward: &Adjacency,
    reverse: &Adjacency,
    from_slug: &str,
    to_slug: &str,
    max_depth: usize,
    max_chains: usize,
) -> Chains {
    if from_slug == to_slug {
        return Chains {
            chains: vec![vec![from_slug.to_string()]],
            truncated: false,
        };
    }
    if max_depth == 0 {
        return Chains {
            chains: Vec::new(),
            truncated: false,
        };
    }
    let distance = distances(forward, from_slug, to_slug, max_depth - 1);
    let empty = Vec::new();
    let mut candidates: Vec<Vec<String>> = Vec::new();
    for caller in reverse.get(to_slug).unwrap_or(&empty) {
        if !distance.contains_key(caller.as_str()) {
            continue;
        }
        let on_paths = ancestors(reverse, &distance, caller);
        let mut prefix = vec![from_slug.to_string()];
        let mut found = Vec::new();
        enumerate(
            forward,
            &distance,
            &on_paths,
            caller,
            &mut prefix,
            max_chains + 1,
            &mut found,
        );
        for mut chain in found {
            chain.push(to_slug.to_string());
            candidates.push(chain);
        }
    }
    candidates.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    let truncated = candidates.len() > max_chains;
    candidates.truncate(max_chains);
    Chains {
        chains: candidates,
        truncated,
    }
}

/// BFS distances from `source` up to `max_distance`, never expanding the
/// target (a chain ends at the target).
fn distances<'a>(
    forward: &'a Adjacency,
    source: &'a str,
    target: &str,
    max_distance: usize,
) -> HashMap<&'a str, usize> {
    let mut distance: HashMap<&str, usize> = HashMap::new();
    distance.insert(source, 0);
    let mut frontier: Vec<&str> = vec![source];
    for depth in 1..=max_distance {
        let mut next: BTreeSet<&str> = BTreeSet::new();
        for slug in &frontier {
            for callee in forward.get(*slug).into_iter().flatten() {
                if callee != target && !distance.contains_key(callee.as_str()) {
                    next.insert(callee.as_str());
                }
            }
        }
        if next.is_empty() {
            break;
        }
        for slug in &next {
            distance.insert(slug, depth);
        }
        frontier = next.into_iter().collect();
    }
    distance
}

/// The functions on a shortest path from the source to `node`.
fn ancestors<'a>(
    reverse: &'a Adjacency,
    distance: &HashMap<&str, usize>,
    node: &'a str,
) -> HashSet<&'a str> {
    let mut seen: HashSet<&str> = HashSet::new();
    seen.insert(node);
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        let Some(&d) = distance.get(current) else {
            continue;
        };
        for parent in reverse.get(current).into_iter().flatten() {
            if d > 0
                && distance.get(parent.as_str()) == Some(&(d - 1))
                && seen.insert(parent.as_str())
            {
                stack.push(parent.as_str());
            }
        }
    }
    seen
}

/// Depth-first, in slug order, over the shortest-path DAG restricted to the
/// functions on a shortest path to `end`: every branch reaches `end`, so the
/// first `limit` chains found are the `limit` smallest in slug order.
fn enumerate(
    forward: &Adjacency,
    distance: &HashMap<&str, usize>,
    on_paths: &HashSet<&str>,
    end: &str,
    prefix: &mut Vec<String>,
    limit: usize,
    found: &mut Vec<Vec<String>>,
) {
    if found.len() >= limit {
        return;
    }
    let Some(current) = prefix.last().cloned() else {
        return;
    };
    if current == end {
        found.push(prefix.clone());
        return;
    }
    let depth = distance
        .get(current.as_str())
        .copied()
        .unwrap_or(usize::MAX);
    for callee in forward.get(&current).into_iter().flatten() {
        if found.len() >= limit {
            return;
        }
        if on_paths.contains(callee.as_str()) && distance.get(callee.as_str()) == Some(&(depth + 1))
        {
            prefix.push(callee.clone());
            enumerate(forward, distance, on_paths, end, prefix, limit, found);
            prefix.pop();
        }
    }
}

/// A node reachable from the BFS source, annotated with its hop distance.
pub struct ReachableNode {
    /// The function slug.
    pub slug: String,
    /// Hop distance from the source function (1 = direct, 2+ = transitive).
    pub depth: u32,
}

/// Every node reachable from `source_slug` within `max_depth` hops, ordered
/// by depth and then by slug. The source node itself is not included.
///
/// Used by `ast_callers` (over the reverse adjacency) and `ast_callees`
/// (over the forward adjacency), which return the first results of this
/// order and its length as their total (KGRAPH-061, KGRAPH-077).
pub fn find_all_reachable(
    adjacency: &Adjacency,
    source_slug: &str,
    max_depth: usize,
) -> Vec<ReachableNode> {
    let mut visited: HashSet<&str> = HashSet::new();
    visited.insert(source_slug);
    let mut frontier: Vec<&str> = vec![source_slug];
    let mut results: Vec<ReachableNode> = Vec::new();
    for depth in 1..=max_depth {
        let mut next: BTreeSet<&str> = BTreeSet::new();
        for slug in &frontier {
            for neighbour in adjacency.get(*slug).into_iter().flatten() {
                if !visited.contains(neighbour.as_str()) {
                    next.insert(neighbour.as_str());
                }
            }
        }
        if next.is_empty() {
            break;
        }
        for slug in &next {
            visited.insert(slug);
            results.push(ReachableNode {
                slug: slug.to_string(),
                depth: depth as u32,
            });
        }
        frontier = next.into_iter().collect();
    }
    results
}
