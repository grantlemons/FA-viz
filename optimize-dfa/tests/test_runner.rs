use optimize_dfa::optimize_transition_table;
use transition_tables::TransitionTable;

macro_rules! compare_optimized {
    ($name:ident, $input_file:literal, $expected_file:literal) => {
        #[test]
        fn $name() {
            let input = include_str!($input_file);
            let expected = include_str!($expected_file);

            // Parse the transition table
            let transition_table = TransitionTable::parse(input).unwrap();

            // Optimize the transition table
            let optimized_transition_table = optimize_transition_table(&transition_table);

            // Serialize the optimized transition table
            let serialized = optimized_transition_table.serialize().unwrap();

            assert_eq!(serialized.trim(), expected.trim());
        }
    };
}

compare_optimized!(zero, "test-0-input.txt", "./test-0-expected-output.txt");
compare_optimized!(one, "test-1-input.txt", "./test-1-expected-output.txt");
compare_optimized!(two, "test-2-input.txt", "./test-2-expected-output.txt");
compare_optimized!(three, "test-3-input.txt", "./test-3-expected-output.txt");
