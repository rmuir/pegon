use tree_sitter::{Query, QueryMatch, QueryPredicateArg};

/// Implement matching for custom predicates
pub fn custom_predicate(
    hit: &QueryMatch,
    data: &[u8],
    operator: &str,
    args: &[QueryPredicateArg],
) -> bool {
    match operator {
        "lt?" => {
            debug_assert!(args.len() > 1);
            let (QueryPredicateArg::Capture(left), QueryPredicateArg::Capture(right)) =
                (&args[0], &args[1])
            else {
                panic!("invalid predicate arguments")
            };
            let node1 = hit
                .nodes_for_capture_index(*left)
                .next()
                .expect("valid capture");
            let node2 = hit
                .nodes_for_capture_index(*right)
                .next()
                .expect("valid capture");
            let slice1 = &data[node1.start_byte()..node1.end_byte()];
            let slice2 = &data[node2.start_byte()..node2.end_byte()];
            slice1 < slice2
        }
        "eol?" => {
            debug_assert!(args.len() == 1);
            let QueryPredicateArg::Capture(left) = &args[0] else {
                panic!("invalid predicate arguments")
            };
            let node = hit
                .nodes_for_capture_index(*left)
                .next()
                .expect("valid capture");
            *data.get(node.end_byte()).unwrap_or(&b'\n') == b'\n'
        }
        _ => {
            panic!("{operator}");
        }
    }
}

/// Returns id of the capture, or panics if it doesn't exist in the query
pub fn capture_id(query: &Query, name: &str) -> u32 {
    query
        .capture_index_for_name(name)
        .unwrap_or_else(|| panic!("{name} capture should exist"))
}
