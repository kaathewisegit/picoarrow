use flatbuffers::{FlatBufferBuilder, WIPOffset};

use crate::{bitmap::ValidityBuffer, fb::Field};

mod fixed_list;
mod primitive;
mod variable;

pub use fixed_list::ArrayFixedSizeList;
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

	fn serialize_field<'fbb>(
		&self,
		builder: &mut FlatBufferBuilder<'fbb>,
		name: &str,
	) -> WIPOffset<Field<'fbb>>;

	fn walk_buffers<F>(&self, f: F)
	where
		F: FnMut(&[u8]);
}
