use alphabet_encoding::{decode, encode};
use std::error::Error;
use std::fmt::Display;
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

mod dfa;
mod to_dfa;

pub use dfa::*;

type State = usize;
type Transitions = BTreeMap<Transition, BTreeSet<State>>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Transition {
    Char(char),
    Lambda,
}

impl Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Transition::Char(c) if *c == ' ' => "SP".to_string(),
            Transition::Char(c) if c.is_ascii_graphic() => c.to_string(),
            Transition::Char(c) => encode(c.to_string()),
            Transition::Lambda => "&lambda;".to_string(),
        };

        write!(f, "{}", s)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct NFA {
    states: BTreeMap<State, (bool, Transitions)>,
    /// Ordering should be preserved and used as the order in the output DFA
    alphabet: Vec<char>,
}

#[derive(Debug)]
pub enum ParseError {
    EmptyFile,
    InvalidFirstLine,
    EmptyAlphabetChar,
    ColumnMismatch,
    EmptyTransition,
    InvalidEncoding,
    InvalidFromTo,
}
impl Error for ParseError {}
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match *self {
            ParseError::EmptyFile => "EmptyFile: File has no lines!",
            ParseError::InvalidFirstLine => {
                "InvalidFirstLine: Unable to parse first line of definition file!"
            }
            ParseError::EmptyAlphabetChar => {
                "EmptyAlphabetChar: Alphabet character is empty string!"
            }
            ParseError::ColumnMismatch => "ColumnMismatch: Not enough columns in row!",
            ParseError::EmptyTransition => "EmptyTransition: Transition character is empty string!",
            ParseError::InvalidEncoding => "InvalidTransition: Decoding alphabet_encoding failed!",
            ParseError::InvalidFromTo => "InvalidFromTo: Unable to parse from or to nodes!",
        };
        write!(f, "{}", str)
    }
}

impl NFA {
    pub fn states(&self) -> BTreeMap<State, (bool, Transitions)> {
        self.states.clone()
    }
}

impl FromStr for NFA {
    type Err = ParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut lines = s.lines();
        let line_fields: Vec<&str> = lines
            .next()
            .ok_or(ParseError::EmptyFile)?
            .splitn(3, " ")
            .collect();
        let (lambda_char, alphabet) = (
            *line_fields.get(1).ok_or(ParseError::InvalidFirstLine)?,
            *line_fields.get(2).ok_or(ParseError::InvalidFirstLine)?,
        );

        // Ordering should be preserved and used as the order in the output DFA
        let alphabet: Result<Vec<char>, ParseError> = decode(alphabet.to_string())
            .map_err(|_| ParseError::InvalidEncoding)?
            .split_whitespace()
            .map(|s| s.chars().next().ok_or(ParseError::EmptyAlphabetChar))
            .collect();

        type DefinitionRow = (bool, State, State, Vec<Transition>);
        let rows: Result<Vec<DefinitionRow>, ParseError> = lines
            .filter(|l| !l.is_empty())
            .map(|l| {
                l.split_whitespace()
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .reduce(|acc, s| acc + " " + &s)
                    .unwrap()
            })
            .map(|l: String| {
                let line_fields: Vec<_> = l.splitn(3, ' ').collect();
                let (accepting, from, to_and_chars) = (
                    *line_fields.first().ok_or(ParseError::ColumnMismatch)?,
                    *line_fields.get(1).ok_or(ParseError::ColumnMismatch)?,
                    *line_fields.get(2).ok_or(ParseError::ColumnMismatch)?,
                );
                let (to, chars) = to_and_chars
                    .split_once(' ')
                    .unwrap_or((to_and_chars, lambda_char));

                let transitions: Result<Vec<_>, ParseError> = chars
                    .to_string()
                    .split_whitespace()
                    .map(|c| decode(c.to_string()).map_err(|_| ParseError::InvalidEncoding))
                    .map(|c| {
                        Ok(match c? {
                            a if a == lambda_char => Transition::Lambda,
                            a => Transition::Char(
                                a.chars().next().ok_or(ParseError::EmptyTransition)?,
                            ),
                        })
                    })
                    .collect();

                Ok((
                    accepting == "+",
                    from.parse::<usize>()
                        .map_err(|_| ParseError::InvalidFromTo)?,
                    to.parse::<usize>().map_err(|_| ParseError::InvalidFromTo)?,
                    transitions?,
                ))
            })
            .collect();

        let mut states: BTreeMap<State, (bool, Transitions)> = BTreeMap::new();
        rows?.into_iter().for_each(|(accepting, from, to, chars)| {
            states
                .entry(from)
                .or_insert((accepting, Transitions::default()))
                .0 |= accepting;
            chars.iter().for_each(|c| {
                states
                    .entry(from)
                    .or_insert((accepting, Transitions::default()))
                    .1
                    .entry(*c)
                    .or_default()
                    .insert(to);
            });
        });

        Ok(Self {
            states,
            alphabet: alphabet?,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::DFA;

    use super::NFA;

    #[test]
    fn parse_example() {
        let src = "13 # * / P
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
+ 12 12";

        let nfa: NFA = src.parse().unwrap();

        assert_eq!(nfa.states.len(), 13);

        let start_to_states: Vec<_> = nfa
            .states
            .get(&0)
            .expect("State not in NFA")
            .1
            .values()
            .cloned()
            .flatten()
            .collect();
        assert_eq!(&start_to_states, &[1]);

        let twelve_to_states: Vec<_> = nfa
            .states
            .get(&12)
            .expect("State not in NFA")
            .1
            .values()
            .cloned()
            .flatten()
            .collect();
        assert_eq!(&twelve_to_states, &[12]);
    }

    #[test]
    fn match_d() {
        let src = "5 # a b c d e f g
+ 0 0 c g f # e b
+ 0 100 f
+ 0 101 e # d g f a
+ 0 102 b e d c g a
+ 0 103 b # d a e f
- 100 0 g 
- 100 100 f 
- 100 101 # b f c a d
- 100 102 c f g d b e
- 100 103 f d c g a e
- 101 100 f 
- 101 101 c g b e d a
- 101 102 d c a # b g
- 101 103 c d # b a e
- 102 100 f
- 102 101 d b c a # g
- 102 102 # d c b a f
- 102 103 f e d c # b
- 103 100 f 
- 103 101 b a f c # g
- 103 102 e a b d c f
- 103 103 g # e f a b
";
        let expected_tt = "+ 0 1 0 0 1 0 0 0
- 1 1 1 1 1 1 2 1
- 2 1 1 1 1 1 2 0
";

        let nfa: NFA = src.parse().unwrap();
        let dfa: DFA = DFA::from(nfa);

        assert_eq!(dfa.ttable().serialize().unwrap(), expected_tt);
    }
}
