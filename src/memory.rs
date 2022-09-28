//! This modules defines a trait for *very simplified* communication between the CPU and
//! "the outside world".
//!
//! See [`crate::cpu`]'s module description for details on the simplification.

/// How the CPU communicates with the "external world".
pub trait AddressSpace {
	/// A read from the address space; reads do not fail (the CPU always gets something).
	fn read(&self, address: u16) -> u8;
	/// A write to the address space.
	fn write(&mut self, address: u16, value: u8);
}
