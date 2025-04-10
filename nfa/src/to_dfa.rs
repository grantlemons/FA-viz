use crate::dfa::DFA;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

use crate::{Transition, NFA};
use dfs::can_reach;
use transition_tables::{TransitionTable, TransitionTableRow, STARTING_STATE_ID};

type State = usize;

impl NFA {
    pub fn lambda_set(&self, state: State) -> Option<BTreeSet<&State>> {
        let mut res = BTreeSet::new();
        let mut cur = self.transition_set(state, &Transition::Lambda)?;

        while !cur.is_empty() && !cur.is_subset(&res) {
            res = res.union(&cur).cloned().collect();
            cur = self
                .transition_set_coll(&res, &Transition::Lambda)
                .unwrap_or(cur);
        }

        Some(res)
    }
    pub fn lambda_set_coll(&self, states: &BTreeSet<&State>) -> Option<BTreeSet<&State>> {
        let mut res = BTreeSet::new();
        let mut cur = self.transition_set_coll(states, &Transition::Lambda)?;

        while !cur.is_empty() && !cur.is_subset(&res) {
            res = res.union(&cur).cloned().collect();
            cur = self
                .transition_set_coll(&res, &Transition::Lambda)
                .unwrap_or(cur);
        }

        Some(res)
    }
    pub fn transition_set(
        &self,
        state: State,
        transition: &Transition,
    ) -> Option<BTreeSet<&State>> {
        Some(self.states.get(&state)?.1.get(transition)?.iter().collect())
    }
    pub fn transition_set_coll(
        &self,
        states: &BTreeSet<&State>,
        transition: &Transition,
    ) -> Option<BTreeSet<&State>> {
        if !states
            .iter()
            .any(|&&s| self.transition_set(s, transition).is_some())
        {
            return None;
        }
        Some(
            states
                .iter()
                .filter_map(|&&s| self.transition_set(s, transition))
                .flatten()
                .collect(),
        )
    }
    pub fn transitions(&self, state: State) -> Option<BTreeSet<&Transition>> {
        Some(self.states.get(&state)?.1.keys().collect())
    }
    pub fn transitions_coll(&self, states: &BTreeSet<&State>) -> Option<BTreeSet<&Transition>> {
        if states.iter().any(|&&s| self.transitions(s).is_none()) {
            return None;
        }
        Some(
            states
                .iter()
                .flat_map(|&&s| self.transitions(s).unwrap())
                .collect(),
        )
    }
    pub fn accepting(&self, state: State) -> Option<bool> {
        Some(self.states.get(&state)?.0)
    }
    pub fn accepting_coll(&self, states: &BTreeSet<&State>) -> bool {
        states
            .iter()
            .filter_map(|&&s| self.accepting(s))
            .any(|bool| bool)
    }
}

impl From<NFA> for DFA {
    fn from(nfa: NFA) -> Self {
        type Row<'a> = (State, bool, Vec<Option<BTreeSet<&'a State>>>);
        type Rows<'a> = BTreeMap<BTreeSet<&'a State>, Row<'a>>;

        let mut traversed: BTreeSet<BTreeSet<&State>> = BTreeSet::new();
        let mut states_queue: VecDeque<BTreeSet<&State>> = VecDeque::new();
        let mut rows: Rows = BTreeMap::new();

        let k = nfa.states.keys().min().expect("No states in NFA");
        states_queue.push_back(
            nfa.lambda_set(*k)
                .into_iter()
                .flatten()
                .chain([k])
                .collect(),
        );

        let mut row_id = STARTING_STATE_ID;
        while let Some(current_states) = states_queue.pop_front() {
            traversed.insert(current_states.clone());

            let transitions: Vec<Option<BTreeSet<&State>>> = nfa
                .alphabet
                .iter()
                .map(|c| nfa.transition_set_coll(&current_states, &Transition::Char(*c)))
                .map(|s| {
                    Some(
                        nfa.lambda_set_coll(&s.clone()?)
                            .into_iter()
                            .flatten()
                            .chain(s?)
                            .collect(),
                    )
                })
                .collect();

            transitions
                .iter()
                .flatten()
                .filter(|&s| !traversed.contains(s))
                .cloned()
                .for_each(|s| {
                    states_queue.push_back(s);
                });

            rows.entry(current_states.clone()).or_insert_with(|| {
                row_id += 1;
                (row_id - 1, nfa.accepting_coll(&current_states), transitions)
            });
        }

        let mut rows: Vec<_> = rows
            .iter()
            .filter(|&(_, row)|
            // filter dead states
                can_reach(
                    row,
                    |r| {
                        r.2.iter().flatten().filter_map(|s| rows.get(s))
                    },
                    |r| r.1,
                ))
            .filter(|&(_, row)|
                // filter unreachable states
                can_reach(
                    rows.values().find(|v| v.0 == 0).expect("No state with id 0"),
                    |r| {
                        r.2.iter().flatten().filter_map(|s| rows.get(s))
                    },
                    |r| r.0 == row.0,
                ))
            .collect();
        rows.sort_unstable_by_key(|(_, (id, _, _))| id);
        rows.sort_by_key(|(_, (_, accepting, _))| accepting);

        let rows: Rows = rows
            .into_iter()
            .enumerate()
            .map(|(i, (k, (_, accepting, transitions)))| {
                (k.clone(), (i, *accepting, transitions.clone()))
            }) // renumber
            .collect();

        let indexes: BTreeMap<char, usize> = nfa
            .alphabet
            .iter()
            .enumerate()
            .map(|(i, &c)| (c, i))
            .collect();

        let mut row_values: Vec<_> = rows.values().collect();
        row_values.sort_unstable_by_key(|(id, _, _)| id);
        let raw_ttable = TransitionTable {
            rows: row_values
                .into_iter()
                .map(|(id, accepting, transitions)| TransitionTableRow {
                    accepting: *accepting,
                    id: *id,
                    transitions: transitions
                        .iter()
                        .map(|t| Some(rows.get(&t.clone()?)?.0))
                        .collect(),
                })
                .collect(),
        };
        Self::new(
            0,
            "".to_string(),
            None,
            Arc::new(indexes),
            optimize_dfa::optimize_transition_table(&raw_ttable),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{DFA, NFA};

    const EXAMPLE_SRC: &str = "13 # * / P
- 0 1 /
- 1 2 *
- 2 3 #
- 2 5 #
- 2 7 #
- 2 10 #
- 3 4 /
- 4 10 #
- 5 6 P
- 6 10 #
- 7 8 *
- 8 8 *
- 8 9 P
- 9 10 #
- 10 2 #
- 10 11 *
- 11 11 *
- 11 12 /
+ 12 12
";
    const EXAMPLE_EXPECTED: &str = "- 0 E 1 E
- 1 2 E E
- 2 3 2 2
- 3 3 4 2
+ 4 E E E
";

    fn view_dfa(dfa: &DFA) {
        println!("{}", dfa.ttable().serialize().unwrap());
    }

    #[test]
    fn example() {
        let nfa: NFA = EXAMPLE_SRC.parse().unwrap();
        let dfa: DFA = nfa.into();

        view_dfa(&dfa);
        assert_eq!(dfa.ttable().serialize().unwrap(), EXAMPLE_EXPECTED);
    }

    #[test]
    fn matcha() {
        let matcha_source = "6 # a b c d e
- 0  10 a b
- 0  11 a b
- 0  12 #
- 10 12 a b c
- 11 11 #
- 11 14 c
- 12 12 c
- 12 0  a
- 12 14 e d c
- 13 13 e
- 13 14 b
+ 14 14 a
+ 14 13 d
";
        let _matcha_expected = "-  0  1  2  6  7  7
-  1  1  4  6  7  7
-  2  5  5  6  E  E
-  3  E  7  E  E  3
-  4  0  5  6  7  7
-  5  0  E  6  7  7
+  6  8  E  6  9  7
+  7  7  E  E  3  E
+  8 10  2  6  9  7
+  9  7  7  E  3  3
+ 10 10  4  6  9  7
";

        let nfa: NFA = matcha_source.parse().unwrap();

        let dfa: DFA = nfa.into();
        view_dfa(&dfa);
        assert_eq!(dfa.ttable().rows.len(), 11);
    }

    #[test]
    fn lambda_set_10() {
        let nfa: NFA = EXAMPLE_SRC.parse().unwrap();

        let l_set = nfa
            .lambda_set(10)
            .expect("Cannot get lambda set of state 10");
        dbg!(&l_set);

        assert_eq!(l_set, BTreeSet::from([&2, &3, &5, &7, &10]));
    }
}
