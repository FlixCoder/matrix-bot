//! Tests for commands

use clap::CommandFactory;

use super::*;

#[test]
fn argument_parsing() {
	let args = parse_arguments("a  bb  ccc");
	assert_eq!(args, vec!["a", "bb", "ccc"]);

	let args = parse_arguments("a 'bb ccc'");
	assert_eq!(args, vec!["a", "bb ccc"]);

	let args = parse_arguments("\"a 'bb\" ccc'");
	assert_eq!(args, vec!["a 'bb", "ccc'"]);

	let args = parse_arguments("aa'bb cc' a\"d\"a");
	assert_eq!(args, vec!["aa'bb", "cc'", "a\"d\"a"]);
}

#[test]
fn clap_verify() {
	Command::command().debug_assert();
}
