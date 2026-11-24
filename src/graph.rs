//! Lays the commit graph out in lanes (columns), one row per commit, newest first.
//!
//! Every lane remembers which commit it is waiting for. A commit takes the first lane that waits
//! for it (or the first free lane if none does, which is a branch tip); other lanes waiting for it
//! join it there. Its first parent keeps the lane; each further parent (a merge) gets its own lane
//! that leaves from the commit's dot. Lanes are colored by column.

use crate::git::Commit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// Stays in its lane.
    Straight,
    /// Leaves a dot sideways, then runs down its lane.
    BranchOut,
    /// Runs down its lane, then turns sideways into the next dot.
    MergeIn,
}

#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

impl Edge {
    /// The column whose color this line takes: the lane it runs down.
    pub fn color_lane(&self) -> usize {
        match self.kind {
            EdgeKind::Straight | EdgeKind::BranchOut => self.to,
            EdgeKind::MergeIn => self.from,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GraphRow {
    pub lane: usize,
    /// Lines from this row's center down to the next row's center.
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub rows: Vec<GraphRow>,
    pub lanes: usize,
}

pub fn layout(commits: &[Commit]) -> Graph {
    let mut lanes: Vec<Option<&str>> = Vec::new();
    // Lanes the previous commit started (fresh) or joined sideways (joins).
    let mut fresh: Vec<bool> = Vec::new();
    let mut joins: Vec<usize> = Vec::new();
    let mut prev_lane = 0usize;
    let mut rows: Vec<GraphRow> = Vec::with_capacity(commits.len());
    let mut max_lanes = 0usize;

    for (i, c) in commits.iter().enumerate() {
        let waiting: Vec<usize> = lanes
            .iter()
            .enumerate()
            .filter(|(_, l)| **l == Some(c.hash.as_str()))
            .map(|(k, _)| k)
            .collect();
        let lane = match waiting.first() {
            Some(&k) => k,
            None => match lanes.iter().position(|l| l.is_none()) {
                Some(k) => k,
                None => {
                    lanes.push(None);
                    fresh.push(false);
                    lanes.len() - 1
                }
            },
        };

        // Lines from the previous row into this one.
        if i > 0 {
            let mut edges = Vec::new();
            for (k, l) in lanes.iter().enumerate() {
                let Some(h) = l else { continue };
                let from = if fresh[k] { prev_lane } else { k };
                let to = if *h == c.hash { lane } else { k };
                edges.push(make_edge(from, to, fresh[k], prev_lane));
            }
            for &k in &joins {
                let to = if lanes[k] == Some(c.hash.as_str()) {
                    lane
                } else {
                    k
                };
                edges.push(make_edge(prev_lane, to, true, prev_lane));
            }
            rows[i - 1].edges = edges;
        }

        for &k in &waiting {
            lanes[k] = None;
        }
        for f in fresh.iter_mut() {
            *f = false;
        }
        joins.clear();

        let mut parents = c.parents.iter();
        if let Some(p0) = parents.next() {
            lanes[lane] = Some(p0.as_str());
        }
        for p in parents {
            if let Some(k) = lanes.iter().position(|l| *l == Some(p.as_str())) {
                if k != lane && !joins.contains(&k) {
                    joins.push(k);
                }
                continue;
            }
            let k = match lanes
                .iter()
                .enumerate()
                .position(|(k, l)| l.is_none() && k != lane)
            {
                Some(k) => k,
                None => {
                    lanes.push(None);
                    fresh.push(false);
                    lanes.len() - 1
                }
            };
            lanes[k] = Some(p.as_str());
            fresh[k] = true;
        }

        let used = lanes
            .iter()
            .rposition(|l| l.is_some())
            .map(|k| k + 1)
            .unwrap_or(0);
        max_lanes = max_lanes.max(used).max(lane + 1);
        prev_lane = lane;
        rows.push(GraphRow {
            lane,
            edges: Vec::new(),
        });
    }

    // History that wasn't loaded: lines run off the bottom.
    if let Some(last) = rows.last_mut() {
        let mut edges = Vec::new();
        for (k, l) in lanes.iter().enumerate() {
            if l.is_some() {
                let from = if fresh[k] { prev_lane } else { k };
                edges.push(make_edge(from, k, fresh[k], prev_lane));
            }
        }
        for &k in &joins {
            edges.push(make_edge(prev_lane, k, true, prev_lane));
        }
        last.edges = edges;
    }

    Graph {
        rows,
        lanes: max_lanes.max(1),
    }
}

fn make_edge(from: usize, to: usize, from_node: bool, node_lane: usize) -> Edge {
    let kind = if from == to {
        EdgeKind::Straight
    } else if from_node && from == node_lane {
        EdgeKind::BranchOut
    } else {
        EdgeKind::MergeIn
    };
    Edge { from, to, kind }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(h: &str, p: &[&str]) -> Commit {
        Commit {
            hash: h.into(),
            parents: p.iter().map(|s| s.to_string()).collect(),
            author: "a".into(),
            email: String::new(),
            date: String::new(),
            subject: String::new(),
        }
    }

    #[test]
    fn merge_and_branch() {
        // M merges B into A; A and B both come from R.
        let commits = vec![
            c("M", &["A", "B"]),
            c("B", &["R"]),
            c("A", &["R"]),
            c("R", &[]),
        ];
        let g = layout(&commits);
        assert_eq!(g.rows[0].lane, 0);
        assert_eq!(g.rows[1].lane, 1);
        assert_eq!(g.rows[2].lane, 0);
        assert_eq!(g.rows[3].lane, 0);
        assert_eq!(g.lanes, 2);
        assert!(g.rows[0]
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::BranchOut && e.to == 1));
        assert!(g.rows[2]
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::MergeIn && e.from == 1));
    }
}
