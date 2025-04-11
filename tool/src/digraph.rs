use itertools::Itertools;
use nfa::NFA;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};
use transition_tables::TransitionTable;

type Node = usize;
#[derive(Default)]
pub struct Digraph {
    pub edges: BTreeMap<(Node, BTreeSet<Node>), String>,
    pub accepting_nodes: BTreeSet<Node>,
}

impl From<&NFA> for Digraph {
    fn from(value: &NFA) -> Self {
        let mut graph = Self::default();

        let states = value.states();
        for (state, (accepting, transitions)) in states {
            for (transition, targets) in transitions {
                graph
                    .edges
                    .entry((state, targets))
                    .and_modify(|acc| {
                        acc.push('|');
                        acc.push_str(&transition.to_string());
                    })
                    .or_insert(transition.to_string());
            }
            if accepting {
                graph.accepting_nodes.insert(state);
            }
        }

        graph
    }
}

impl From<&TransitionTable> for Digraph {
    fn from(value: &TransitionTable) -> Self {
        let mut graph = Self::default();

        for state in &value.rows {
            for (i, t) in state.transitions.iter().enumerate() {
                if let Some(t) = t {
                    let transition: String = char::from_u32(i as u32 + 'a' as u32)
                        .expect("Unable to convert from decimal to char.")
                        .to_string();
                    let destination: Node = *t;
                    graph
                        .edges
                        .entry((state.id, BTreeSet::<Node>::from([destination])))
                        .and_modify(|acc| {
                            acc.push('|');
                            acc.push_str(&transition.to_string());
                        })
                        .or_insert(transition.to_string());
                    if state.accepting {
                        graph.accepting_nodes.insert(state.id);
                    }
                }
            }
        }

        graph
    }
}

impl Display for Digraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node_defs: String = Itertools::intersperse(
            self.accepting_nodes
                .iter()
                .map(|n| format!("{} [shape=doublecircle]", n)),
            "\n".to_owned(),
        )
        .collect();
        let edge_defs: String = Itertools::intersperse(
            self.edges.iter().map(|((src, destinations), label)| {
                format!(
                    "{} -> {{ {} }} [label=<{}>]",
                    src.to_string(),
                    destinations
                        .iter()
                        .map(|n| n.to_string())
                        .reduce(|acc, n| acc + "," + &n)
                        .unwrap(),
                    label,
                )
            }),
            "\n".to_owned(),
        )
        .collect();
        write!(
            f,
            "digraph {{
newrank=true;
rankdir=LR;
{}
{}
}}",
            node_defs, edge_defs
        )
    }
}
