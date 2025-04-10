use dfs::can_reach;
use optimize_dfa::optimize_transition_table;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::RangeInclusive;
use std::sync::Arc;

use transition_tables::TransitionTable;
use transition_tables::TransitionTableRow;

#[derive(Debug, PartialEq)]
pub struct DFAState {
    pub can_accept: bool,
    pub reachable: bool,
    pub tt_row: TransitionTableRow,
}

#[derive(Clone, Debug)]
pub struct DFA {
    pub index: usize,
    pub id: Arc<String>,
    pub associated_value: Arc<Option<String>>,
    state: usize,
    states: Arc<Vec<DFAState>>,
    indexes: Arc<BTreeMap<char, usize>>,
}

pub enum CheckMatchResult {
    Success,
    Failure(usize),
}

impl PartialEq for DFA {
    fn eq(&self, other: &Self) -> bool {
        self.states == other.states
    }
}

impl Display for DFA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] ({}, {})",
            self.id,
            self.accepting(),
            self.can_accept()
        )
    }
}

impl DFA {
    pub fn new(
        index: usize,
        id: String,
        associated_value: Option<String>,
        indexes: Arc<BTreeMap<char, usize>>,
        table: TransitionTable,
    ) -> DFA {
        let reduced = optimize_transition_table(&table);
        let states = reduced
            .rows
            .iter()
            // reduce unreachable states
            .map(|row| DFAState {
                can_accept: can_reach(
                    row.id,
                    |row_id| {
                        reduced.rows[*row_id]
                            .transitions
                            .clone()
                            .into_iter()
                            .flatten()
                    },
                    |row_id| reduced.rows[*row_id].accepting,
                ),
                reachable: can_reach(
                    0,
                    |row_id| {
                        reduced.rows[*row_id]
                            .transitions
                            .clone()
                            .into_iter()
                            .flatten()
                    },
                    |row_id| reduced.rows[*row_id].id == *row_id,
                ),
                tt_row: row.clone(),
            })
            .collect();

        DFA {
            index,
            id: Arc::new(id),
            associated_value: Arc::new(associated_value),
            state: 0,
            states: Arc::new(states),
            indexes,
        }
    }

    pub fn verify_row_lengths(&self) -> bool {
        self.states
            .iter()
            .all(|row| row.tt_row.transitions.len() == self.indexes.len())
    }

    fn current_row(&self) -> &DFAState {
        &self.states[self.state]
    }

    pub fn current_state(&self) -> usize {
        self.state
    }

    pub fn accepting(&self) -> bool {
        self.states[self.state].tt_row.accepting
    }

    pub fn can_accept(&self) -> bool {
        self.states[self.state].can_accept
    }

    pub fn reset(&mut self) {
        self.state = 0;
    }

    pub fn start_state(&self) -> Self {
        Self {
            state: 0,
            ..self.clone()
        }
    }

    pub fn transition(&self, t: &char) -> Option<Self> {
        self.current_row().tt_row.transitions[*self.indexes.get(t)?].map(|state| Self {
            state,
            ..self.clone()
        })
    }

    pub fn transition_mut(&mut self, t: &char) -> Option<usize> {
        self.current_row().tt_row.transitions[*self.indexes.get(t)?].inspect(|&state| {
            self.state = state;
        })
    }

    pub fn ttable(&self) -> TransitionTable {
        TransitionTable {
            rows: self
                .states
                .iter()
                .filter(|s| s.reachable)
                .filter(|s| s.can_accept)
                .map(|s| s.tt_row.clone())
                .collect(),
        }
    }

    pub fn check_match(&self, source: &[char]) -> CheckMatchResult {
        if source.is_empty() && !self.accepting() {
            return CheckMatchResult::Failure(0);
        }

        let (final_state, status) = source.iter().enumerate().fold(
            (self.clone(), CheckMatchResult::Success),
            |(acc, res), (i, c)| match (acc.transition(c), res) {
                (Some(new_dfa), CheckMatchResult::Success) => (new_dfa, CheckMatchResult::Success),
                (None, CheckMatchResult::Success) => (acc, CheckMatchResult::Failure(i + 1)),
                (_, a @ CheckMatchResult::Failure(_)) => (acc, a),
            },
        );
        match status {
            CheckMatchResult::Success if !final_state.accepting() => {
                CheckMatchResult::Failure(source.len() + 1)
            }
            CheckMatchResult::Success => CheckMatchResult::Success,
            f @ CheckMatchResult::Failure(_) => f,
        }
    }
}

#[derive(Debug)]
struct MatchSpan {
    token_id: String,
    associated_value: Option<String>,
    span: RangeInclusive<usize>,
}

#[derive(Debug, Clone)]
pub struct Match<'a> {
    pub token_id: String,
    pub associated_value: Option<String>,
    pub token_value: &'a [char],
    pub span: RangeInclusive<usize>,
    pub line_number: usize,
    pub line_location: usize,
}

impl Display for Match<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.token_id,
            self.associated_value
                .clone()
                .unwrap_or(alphabet_encoding::encode(self.token_value.iter().collect())),
            self.line_number + 1,
            self.line_location,
        )
    }
}

pub fn run_dfas<'a>(dfas: &[DFA], source: &'a [char]) -> Vec<Match<'a>> {
    process_spans(&matches(dfas, source), source)
}

fn next_match(dfas: &[DFA], source: &[char], offset: usize) -> Option<MatchSpan> {
    let mut cursor = offset;
    let mut in_progress_dfas: Vec<DFA> = dfas.to_vec();
    let mut finished_dfas: Vec<(DFA, usize)> = Vec::new();

    while cursor < source.len() && !in_progress_dfas.is_empty() {
        let c = source[cursor];

        // Remove any dfas that do not allow the current character
        in_progress_dfas = in_progress_dfas
            .iter()
            .filter_map(|dfa| dfa.transition(&c))
            .filter(|dfa| dfa.can_accept())
            .collect();

        finished_dfas = in_progress_dfas
            .iter()
            .filter(|dfa| dfa.accepting())
            .cloned()
            .map(|dfa| (dfa, cursor))
            .chain(finished_dfas)
            .collect();

        cursor += 1;
    }

    let longest_dfa_length = *finished_dfas.iter().map(|(_, end)| end).max()?;
    let (best_match, end) = finished_dfas
        .into_iter()
        .filter(|(_, end)| *end == longest_dfa_length)
        .min_by_key(|(d, _)| d.index)?;

    Some(MatchSpan {
        token_id: best_match.id.to_string(),
        associated_value: (*best_match.associated_value).clone(),
        span: offset..=end,
    })
}

fn matches(dfas: &[DFA], source: &[char]) -> Vec<MatchSpan> {
    let mut cursor = 0;
    let mut found = Vec::new();

    while cursor < source.len() {
        let current = next_match(dfas, source, cursor);
        if let Some(m) = current {
            cursor = m.span.end() + 1;
            found.push(m);
        } else {
            return found;
        }
    }

    found
}

fn process_spans<'a>(spans: &[MatchSpan], source: &'a [char]) -> Vec<Match<'a>> {
    // (line_number, start_position)
    let line_positions: Vec<(usize, usize)> = source
        .iter()
        .enumerate()
        .filter(|(_, c)| **c == '\n')
        .map(|(i, _)| i)
        .enumerate()
        .collect();
    spans
        .iter()
        .map(|s| Match::<'a> {
            token_id: s.token_id.clone(),
            associated_value: s.associated_value.clone(),
            token_value: &source[s.span.clone()],
            span: s.span.clone(),
            line_number: line_positions
                .iter()
                .find(|(_, i)| s.span.start() <= i)
                .map_or(line_positions.len(), |(l, _)| *l),
            line_location: line_positions
                .iter()
                .rev()
                .find(|(_, i)| s.span.start() > i)
                .map_or(*s.span.start() + 1, |(_, i)| *s.span.start() - i),
        })
        .collect()
}
