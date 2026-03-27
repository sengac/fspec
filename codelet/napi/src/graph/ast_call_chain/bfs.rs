//! BFS traversal over a call-graph adjacency list.
//!
//! Two traversal modes:
//! - `find_paths`: find all shortest paths between two specific functions (KGRAPH-060)
//! - `find_all_reachable`: find all functions reachable from a source with depth (KGRAPH-061)

use std::collections::{HashMap, HashSet, VecDeque};

/// Edge metadata for a single hop in the call chain.
#[derive(Clone)]
pub struct CallEdgeInfo {
    pub call_count: Option<i64>,
    pub is_conditional: Option<bool>,
}

/// Adjacency entry: callee slug + edge metadata.
pub struct AdjEntry {
    pub callee_slug: String,
    pub edge_info: CallEdgeInfo,
}

/// BFS to find all shortest paths from source to target within depth limit.
///
/// Returns paths as vectors of slugs, ordered by length (shortest first).
pub fn find_paths(
    adj: &HashMap<String, Vec<AdjEntry>>,
    from_slug: &str,
    to_slug: &str,
    max_depth: usize,
    max_chains: usize,
) -> Vec<Vec<String>> {
    let mut all_paths: Vec<Vec<String>> = Vec::new();
    let mut queue: VecDeque<Vec<String>> = VecDeque::new();
    let mut visited_at_depth: HashMap<String, usize> = HashMap::new();

    queue.push_back(vec![from_slug.to_string()]);
    visited_at_depth.insert(from_slug.to_string(), 0);

    while let Some(path) = queue.pop_front() {
        let depth = path.len() - 1;

        if depth > max_depth {
            break;
        }

        if all_paths.len() >= max_chains {
            break;
        }

        let current = match path.last() {
            Some(c) => c,
            None => continue,
        };

        if current == to_slug {
            all_paths.push(path);
            continue;
        }

        if let Some(entries) = adj.get(current) {
            for entry in entries {
                let callee = &entry.callee_slug;

                // Prevent cycles within a single path
                if path.contains(callee) {
                    continue;
                }

                let new_depth = depth + 1;
                if new_depth > max_depth {
                    continue;
                }

                // Allow revisiting a node if we reach it at the same depth
                // (to find multiple paths through different routes).
                // Always allow expanding toward the target to find all paths.
                let should_expand = if callee == to_slug {
                    true
                } else {
                    match visited_at_depth.get(callee) {
                        Some(&prev_depth) => new_depth <= prev_depth,
                        None => true,
                    }
                };

                if should_expand {
                    visited_at_depth.insert(callee.clone(), new_depth);
                    let mut new_path = path.clone();
                    new_path.push(callee.clone());
                    queue.push_back(new_path);
                }
            }
        }
    }

    all_paths.sort_by_key(|p| p.len());
    all_paths.truncate(max_chains);
    all_paths
}

/// A node reachable from the BFS source, annotated with its hop distance.
pub struct ReachableNode {
    /// The function slug.
    pub slug: String,
    /// Hop distance from the source function (1 = direct, 2+ = transitive).
    pub depth: u32,
}

/// BFS to find all nodes reachable from `source_slug` within `max_depth` hops.
///
/// Returns a flat list of [`ReachableNode`] ordered by depth (shallowest first),
/// then by slug within each depth level. The source node itself is NOT included.
///
/// Used by `ast_callers` and `ast_callees` (KGRAPH-061).
pub fn find_all_reachable(
    adj: &HashMap<String, Vec<AdjEntry>>,
    source_slug: &str,
    max_depth: usize,
    max_results: usize,
) -> Vec<ReachableNode> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut frontier: Vec<String> = vec![source_slug.to_string()];
    let mut results: Vec<ReachableNode> = Vec::new();

    visited.insert(source_slug.to_string());

    for depth in 1..=max_depth {
        let mut next_frontier: Vec<String> = Vec::new();

        for slug in &frontier {
            if let Some(entries) = adj.get(slug) {
                for entry in entries {
                    if visited.insert(entry.callee_slug.clone()) {
                        results.push(ReachableNode {
                            slug: entry.callee_slug.clone(),
                            depth: depth as u32,
                        });
                        next_frontier.push(entry.callee_slug.clone());

                        if results.len() >= max_results {
                            return results;
                        }
                    }
                }
            }
        }

        if next_frontier.is_empty() {
            break;
        }
        frontier = next_frontier;
    }

    results
}

/// Build a reverse adjacency list from a forward one.
///
/// For each forward entry `A → B`, creates a reverse entry `B → A`.
/// Used by `ast_callers` to traverse incoming Calls edges without
/// requiring a separate database query.
pub fn reverse_adjacency(
    forward: &HashMap<String, Vec<AdjEntry>>,
) -> HashMap<String, Vec<AdjEntry>> {
    let mut reverse: HashMap<String, Vec<AdjEntry>> = HashMap::new();

    for (caller_slug, callees) in forward {
        for entry in callees {
            reverse
                .entry(entry.callee_slug.clone())
                .or_default()
                .push(AdjEntry {
                    callee_slug: caller_slug.clone(),
                    edge_info: entry.edge_info.clone(),
                });
        }
    }

    reverse
}
