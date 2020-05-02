use std::{error::Error, fmt};

pub use sc2_proc_macro::{bot, bot_new, variant_checkers, FromStr};

#[derive(Debug, PartialEq)]
pub struct ParseEnumError;

impl fmt::Display for ParseEnumError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "failed to parse enum")
	}
}

impl Error for ParseEnumError {}
