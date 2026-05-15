use flatbuffers::{FlatBufferBuilder, WIPOffset};

use crate::{bitmap::Bitmap, fb::Field};

mod primitive;

// TODO: seal
pub trait Validity {
	const IS_NULLABLE: bool;
	type Container;
}

pub struct Nullable;
impl Validity for Nullable {
	const IS_NULLABLE: bool = true;
	type Container = Bitmap;
}

pub struct NonNullable;
impl Validity for NonNullable {
	const IS_NULLABLE: bool = false;
	type Container = ();
}

pub trait Array {
	fn len(&self) -> usize;

	fn is_empty(&self) -> bool;

	fn serialize_field<'fbb>(
		&self,
		builder: &mut FlatBufferBuilder<'fbb>,
		name: &str,
	) -> WIPOffset<Field<'fbb>>;
}
