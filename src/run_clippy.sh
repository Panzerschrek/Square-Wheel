cargo +nightly clippy -- \
-A clippy::precedence \
-A clippy::collapsible_if \
-A clippy::collapsible_else_if \
-A clippy::needless_borrow \
-A clippy::map_entry \
-A clippy::single_match \
-A clippy::needless_range_loop \
-A clippy::new_without_default \
-A clippy::missing_safety_doc \
-A clippy::too_many_arguments \
-A clippy::clone_double_ref \
-A clippy::excessive_precision \
-A clippy::mut_from_ref \
-A clippy::useless_format \
-A clippy::if_same_then_else \
-A clippy::from_over_into \
-A clippy::absurd_extreme_comparisons \
-A clippy::unnecessary_cast \
-A clippy::redundant_closure \
-A clippy::erasing_op \
-A clippy::explicit_counter_loop \
-A clippy::identity_op \
-A clippy::manual_range_contains \
-A clippy::wildcard_in_or_patterns \
-A clippy::field_reassign_with_default \
-A clippy::len_zero \