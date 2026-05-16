//! Arrow-compatible array objects
//!
//! Currently there isn't any interop support besides encoding them into an IPC
//! format.  All arrays own their storage, so they can be grown or shrank,
//! similar to how builders work in `arrow-rs`.
//!
//! The nullability of arrays is set at compile time using the [`Nullable`] and
//! [`NonNullable`] type parameters.  This means non-nullable arrays are
//! smaller, because they don't have the nullability buffer (both in allocation
//! and struct size) and they expose methods which return the values themselves
//! instead of an `Option`.

use crate::{bitmap::ValidityBuffer, schema::Field, seal};

mod boolean;
mod fixed_binary;
mod fixed_list;
mod primitive;
mod variable;

pub use boolean::ArrayBoolean;
pub use fixed_binary::ArrayFixedBinary;
pub use fixed_list::ArrayFixedSizeList;
#[cfg(feature = "half")]
pub use primitive::ArrayF16;
pub use primitive::{
	ArrayF32, ArrayF64, ArrayI8, ArrayI16, ArrayI32, ArrayI64,
	ArrayPrimitive, ArrayU8, ArrayU16, ArrayU32, ArrayU64,
};
pub use variable::{ArrayBinary, ArrayUtf8};

/// A unifying trait for [`Nullable`] and [`NonNullable`]
pub trait Validity: seal::Seal {
	const IS_NULLABLE: bool;
	type Container: ValidityBuffer;
}

/// Marks arrays which might have null items
pub struct Nullable;
impl seal::Seal for Nullable {}
impl Validity for Nullable {
	const IS_NULLABLE: bool = true;
	type Container = Vec<u8>;
}

/// Marks arrays whose items cannot be null
pub struct NonNullable;
impl seal::Seal for NonNullable {}
impl Validity for NonNullable {
	const IS_NULLABLE: bool = false;
	type Container = ();
}

/// A common trait shared by all array implementations
///
/// Note that unlike in `arrow-rs` this trait describes arrays which own their
/// data, so they can be edited (e.g. the [`clear`][`Array::clear`] method).
pub trait Array: seal::Seal {
	/// The number of elements in the array
	///
	/// This counts the number of logical elements.  For example, in
	/// variable length types such as `ArrayUtf8` this will count the number
	/// of strings and not the length of the underlying buffer.  And for
	/// `ArrayBoolean` this will return the number of bits in the array, not
	/// bytes.  Use [`memory_size`][`Array::memory_size`] get the size of
	/// underlying buffers.
	fn len(&self) -> usize;

	/// `true` if there are no items in the array
	fn is_empty(&self) -> bool;

	/// The total number of null items in the array
	fn null_count(&self) -> usize;

	/// Total amount of bytes taken up by initialized values in the
	/// allocated buffers
	///
	/// Note that this function only counts the size of initialized data,
	/// not the size of all underlying allocations, which pre-allocate on
	/// growth.
	fn memory_size(&self) -> usize;

	/// Delete all items in the array
	///
	/// This function is similar to [`Vec::clear`] in that it is cheap and
	/// does not shrink the allocations.
	fn clear(&mut self);

	#[doc(hidden)]
	fn make_field(&self, name: &str) -> Field;

	#[doc(hidden)]
	fn walk_buffers(&self, f: &mut dyn FnMut(&[u8]));

	#[doc(hidden)]
	fn walk_nodes(&self, f: &mut dyn FnMut(usize, usize));
}
