use tree_sitter::{QueryMatch, QueryPredicateArg};

/// Implement matching for custom predicates
pub fn custom_predicate(
    hit: &QueryMatch,
    data: &[u8],
    operator: &str,
    args: &[QueryPredicateArg],
) -> bool {
    match operator {
        "lt?" => {
            assert!(args.len() > 1);
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
            node1.utf8_text(data).unwrap_or_default() < node2.utf8_text(data).unwrap_or_default()
        }
        "eol?" => {
            assert!(args.len() == 1);
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
