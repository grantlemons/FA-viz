use alphabet_encoding::{decode, encode};
use std::error::Error;
use std::fmt::Display;
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

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
