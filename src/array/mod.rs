use crate::{bitmap::ValidityBuffer, schema::Field};

mod fixed_list;
mod primitive;
mod variable;

pub use fixed_list::ArrayFixedSizeList;
pub use primitive::{
	ArrayF32, ArrayF64, ArrayI8, ArrayI16, ArrayI32, ArrayI64, ArrayU8,
	ArrayU16, ArrayU32, ArrayU64,
};
pub use variable::{ArrayBinary, ArrayUtf8};

// TODO: seal
pub trait Validity {
	const IS_NULLABLE: bool;
	type Container: ValidityBuffer;
}

pub struct Nullable;
impl Validity for Nullable {
	const IS_NULLABLE: bool = true;
	type Container = Vec<u8>;
}

pub struct NonNullable;
impl Validity for NonNullable {
	const IS_NULLABLE: bool = false;
	type Container = ();
}

pub trait Array {
	fn len(&self) -> usize;

	fn is_empty(&self) -> bool;

	fn null_count(&self) -> usize;

	fn memory_size(&self) -> usize;

	fn clear(&mut self);

	fn make_field(&self, name: &str) -> Field;

	fn walk_buffers(&self, f: &mut dyn FnMut(&[u8]));

	fn walk_nodes(&self, f: &mut dyn FnMut(usize, usize));
}
