use std::collections::{HashMap, HashSet, VecDeque};
use transition_tables::TransitionTable;

fn partition_states(
    table: &TransitionTable,
    states: &[usize],
    transition: usize,
) -> Vec<Vec<usize>> {
    let mut partitions_by_id: HashMap<(usize, Option<usize>), Vec<usize>> = HashMap::new();

    table
        .rows
        .iter()
        .filter(|r| states.contains(&r.id))
        .for_each(|r| {
            let partition = partitions_by_id
                .entry((transition, r.transitions[transition]))
                .or_default();
            partition.push(r.id);
        });

    partitions_by_id
        .values()
        .map(|partition| {
            let mut states = partition.clone();
            states.sort_unstable();
            states.dedup();
            states
        })
        .collect()
}

/// Merge states of a DFA (Note this should be called repeatedly until no more rows of the table are
/// merged)
fn merge_states(input: &TransitionTable) -> TransitionTable {
    if input.rows.is_empty() {
        return input.clone();
    }

    let mut merged_states: HashSet<Vec<usize>> = HashSet::new();
    let mut merge_queue: VecDeque<(Vec<usize>, Vec<usize>)> = VecDeque::new();
    let alphabet = (0..input.rows[0].transitions.len()).collect::<Vec<_>>();

    merge_queue.push_back((
        input
            .rows
            .iter()
            .filter(|r| r.accepting)
            .map(|row| row.id)
            .collect(),
        alphabet.clone(),
    ));
    merge_queue.push_back((
        input
            .rows
            .iter()
            .filter(|r| !r.accepting)
            .map(|row| row.id)
            .collect(),
        alphabet.clone(),
    ));

    // Identify rows to merge
    while !merge_queue.is_empty() {
        let (states, alphabet) = merge_queue.pop_front().unwrap();

        let (&transition, remaining_alphabet) = alphabet.split_first().unwrap();
        partition_states(input, &states, transition)
            .iter()
            .filter(|x| x.len() > 1)
            .for_each(|x| {
                if remaining_alphabet.is_empty() {
                    merged_states.insert(x.clone());
                } else {
                    merge_queue.push_back((x.clone(), remaining_alphabet.to_vec()));
                }
            });
    }

    let mut output = input.clone();
    for states in merged_states {
        assert!(
            states.len() > 1,
            "Merged states must have at least 2 states"
        );

        let (first_id, rest) = states.split_first().unwrap();

        // Remove the rest of the rows
        for rest_id in rest {
            let row_index = output
                .rows
                .iter()
                .position(|row| row.id == *rest_id)
                .unwrap();
            output.rows.remove(row_index);
        }

        // Update all transitions to the rest to now point to the first
        for row in &mut output.rows {
            // Update transitions
            row.transitions
                .iter_mut()
                .flatten()
                .filter(|state| rest.contains(state))
                .for_each(|state| *state = *first_id);
        }
    }

    output
}

/// Optimize a transition table
pub fn optimize_transition_table(table: &TransitionTable) -> TransitionTable {
    let mut before = table.clone();

    loop {
        let after = merge_states(&before);

        if after.rows.len() == before.rows.len() {
            break;
        } else {
            before = after;
        }
    }

    // renumber rows
    let mut res = before.clone();
    res.rows.sort_unstable_by_key(|r| r.id);
    res.rows
        .clone()
        .into_iter()
        .enumerate()
        .filter(|(i, r)| *i != r.id)
        .for_each(|(i, r)| {
            let old_id = r.id;
            res.rows[i].id = i;
            res.rows.iter_mut().for_each(|inner_row| {
                inner_row
                    .transitions
                    .iter_mut()
                    .flatten()
                    .filter(|t| **t == old_id)
                    .for_each(|id| *id = i)
            });
        });

    res
}
