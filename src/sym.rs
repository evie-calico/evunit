//! This module parses [sym files](https://rgbds.gbdev.io/sym).

use thiserror::Error;

use std::collections::{HashMap, HashSet};
use std::num::ParseIntError;

/// A collection of symbols.
///
/// This is not a [`HashMap`], because:
/// > "A symbol is uniquely defined by the combination of its location and its name"
///
/// ...and thus, a symbol does not necessarily have a unique name.
///
/// If you only plan to refer to symbols by name, consider [`UniquelyNamedSyms`].
pub type Symbols = HashSet<(String, Location)>;
/// This is a convenience for when you expect to only refer to symbols by name. Prefer [`Symbols`] otherwise.
pub type UniquelyNamedSyms = HashMap<String, Location>;

#[derive(Debug, PartialEq, Eq, Hash)]
/// A symbol's location.
pub enum Location {
	/// The symbol refers to a specific memory bank.
	Banked(u32, u16),
	/// The symbol refers to the boot ROM.
	Boot(u16),
	/// The symbol refers to a memory address, but not to a specific bank.
	Unbanked(u16),
}

/// Parses a sym file line.
/// The line should not contain an EOL marker.
pub fn parse_line(line: &str) -> Result<Option<(String, Location)>, ParseError> {
	// Strip the comment, if any; this is OK because they cannot be escaped.
	let mut tokens = line
		.find(';')
		.map_or(line, |pos| &line[..pos])
		.split(|c| matches!(c, ' ' | '\t'));

	let first = match tokens.next() {
		None => return Ok(None), // "A line without any tokens shall be silently ignored"
		Some(token) => token,
	};
	let second = match tokens.next() {
		None => return Ok(None), // "A line with only one token shall be ignored[...]. Encountering one may produce a warning."
		Some(token) => token,
	};
	// "further tokens (if any) may provide extra metadata."

	let location = match first.split_once(':') {
		Some((bank, addr)) => {
			let addr = u16::from_str_radix(addr, 16).map_err(ParseError::BadAddress)?;
			if bank.eq_ignore_ascii_case("BOOT") {
				Location::Unbanked(addr)
			} else {
				Location::Banked(
					u32::from_str_radix(bank, 16).map_err(ParseError::BadBank)?,
					addr,
				)
			}
		}
		None => Location::Unbanked(u16::from_str_radix(first, 16).map_err(ParseError::BadAddress)?),
	};

	// Symbol names must match the regex `[A-Za-z_]([A-Za-z0-9_@#$.]|\\u[A-Za-z0-9]{4}|\\U[A-Za-z0-9]{8})*`
	if let Some(c) = second
		.chars()
		.find(|c| matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '@' | '#' | '$' | '.'))
	{
		return Err(ParseError::BadChar(c));
	}

	let mut name = String::with_capacity(second.len());
	let mut slices = second.split('\\');
	name.push_str(slices.next().unwrap()); // Tokens cannot be empty
	match name.chars().next() {
		Some('A'..='Z' | 'a'..='z' | '_') => (),
		c => return Err(ParseError::BadFirstLetter(c.unwrap_or('\\'))),
	}
	// Each of the remaining slices begins with a character escape
	for slice in slices {
		let mut chars = slice.chars();
		let codep = match chars.next() {
			Some('u') => {
				let digits = ArrayStr::<{ utf8cap(4) }>::new(&mut chars)
					.ok_or(ParseError::TruncatedEscape)?;
				u32::from_str_radix(&digits, 16)
			}
			Some('U') => {
				let digits = ArrayStr::<{ utf8cap(8) }>::new(&mut chars)
					.ok_or(ParseError::TruncatedEscape)?;
				u32::from_str_radix(&digits, 16)
			}
			c => return Err(c.map_or(ParseError::TruncatedEscape, ParseError::BadEscape)),
		}
		.map_err(ParseError::BadCodepoint)?;
		name.push(char::from_u32(codep).ok_or(ParseError::InvalidCodepoint(codep))?);
		name.extend(chars);
	}

	Ok(Some((name, location)))
}

#[derive(Debug, Error)]
/// An error encountered when parsing a sym file line.
pub enum ParseError {
	#[error("bad bank: {0}")]
	BadBank(ParseIntError),
	#[error("bad address: {0}")]
	BadAddress(ParseIntError),
	#[error("'{0}' is not allowed to start a name")]
	BadFirstLetter(char),
	#[error("'{0}' is not allowed in names")]
	BadChar(char),
	#[error("'{0}' cannot be escaped")]
	BadEscape(char),
	#[error("not enough character after escape sequence")]
	TruncatedEscape,
	#[error("bad escape sequence: {0}")]
	BadCodepoint(ParseIntError),
	#[error("invalid codepoint U+{0:x}")]
	InvalidCodepoint(u32),
}

mod array_str {
	use std::ops::Deref;

	const MOST_BYTES_PER_CHARACTER: usize = 4;
	pub const fn utf8cap(n: usize) -> usize {
		n * MOST_BYTES_PER_CHARACTER
	}
	/// Careful, the const argument is in **bytes**, not `char`s.
	/// You should pass `utf8cap(nb_chars)`.
	pub struct ArrayStr<const N: usize>([u8; N], usize);

	impl<const N: usize> ArrayStr<N> {
		pub fn new<It: Iterator<Item = char>>(iter: &mut It) -> Option<Self> {
			let mut this = Self([0; N], 0);
			for _ in 0..N {
				let c = iter.next()?;
				debug_assert!(this.0.len() - this.1 >= MOST_BYTES_PER_CHARACTER); // Thus, below should never fail
				this.1 += c.encode_utf8(&mut this.0[this.1..]).as_bytes().len();
			}
			Some(this)
		}
	}
	impl<const N: usize> Deref for ArrayStr<N> {
		type Target = str;
		fn deref(&self) -> &Self::Target {
			// Safety: all bytes originate from non-overlapping `encode_utf8`s.
			unsafe { std::str::from_utf8_unchecked(&self.0[..self.1]) }
		}
	}
}
use array_str::{utf8cap, ArrayStr};
