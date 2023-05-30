warnings_to_igore=(
	# Collapsable if warning is stupid - fixing it makes if-else chanes messy.
	-A clippy::collapsible_else_if
	-A clippy::collapsible_if
	# Sometimes multiplying by 0 has sense - for better clarity.
	-A clippy::erasing_op
	-A clippy::excessive_precision
	# Sometimes it is necessary to use counter of type i32, u32, etc., but not "usize", like with "enumerate" method.
	-A clippy::explicit_counter_loop
	# Sometimes multiplying by 1 has sense - for better clarity.
	-A clippy::identity_op
	# Sometimes it is better to check vector size against 1 (for example, for list of args).
	-A clippy::len_zero
	# Using method "constains" is ugly.
	-A clippy::manual_range_contains
	# Do not care about safety docs. Unsafe code is used only internally.
	-A clippy::missing_safety_doc
	-A clippy::needless_range_loop
	-A clippy::new_without_default
	# Dummy check - sometimes it is necessary to have a lot of args.
	-A clippy::too_many_arguments
	-A clippy::wildcard_in_or_patterns
)

cargo +nightly clippy -- "${warnings_to_igore[@]}"