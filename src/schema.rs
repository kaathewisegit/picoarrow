use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use crate::array::Array;
use crate::fb::{
	Binary, BinaryArgs, BinaryView, BinaryViewArgs, Bool, BoolArgs, Date,
	DateArgs, DateUnit, Decimal, DecimalArgs, Duration, DurationArgs,
	Endianness, Feature, Field as FbField, FieldArgs, FixedSizeBinary,
	FixedSizeBinaryArgs, FixedSizeList, FixedSizeListArgs, FloatingPoint,
	FloatingPointArgs, Int, IntArgs, Interval, IntervalArgs, IntervalUnit,
	LargeBinary, LargeBinaryArgs, LargeList, LargeListArgs, LargeListView,
	LargeListViewArgs, LargeUtf8, LargeUtf8Args, List, ListArgs, ListView,
	ListViewArgs, Map, MapArgs, Null, NullArgs, Precision, RunEndEncoded,
	RunEndEncodedArgs, Schema as FbSchema, SchemaArgs, Time, TimeArgs,
	TimeUnit, Timestamp, TimestampArgs, Type, Utf8, Utf8Args, Utf8View,
	Utf8ViewArgs,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
	fields: Vec<Field>,
}

impl Schema {
	pub fn from_fields<I>(fields: I) -> Self
	where
		I: IntoIterator<Item = Field>,
	{
		Self {
			fields: fields.into_iter().collect(),
		}
	}

	pub fn from_arrays<'a, I>(arrays: I) -> Self
	where
		I: IntoIterator<Item = (&'a str, &'a dyn Array)>,
	{
		let fields = arrays
			.into_iter()
			.map(|(name, array)| array.make_field(name))
			.collect();

		Self { fields }
	}

	pub(crate) fn serialize<'fbb>(
		&self,
		builder: &mut FlatBufferBuilder<'fbb>,
	) -> WIPOffset<FbSchema<'fbb>> {
		let fields: Vec<_> = self
			.fields
			.iter()
			.map(|field| field.create_field(builder))
			.collect();

		let fields = builder.create_vector(&fields);
		let features =
			builder.create_vector(&[Feature::COMPRESSED_BODY]);

		FbSchema::create(
			builder,
			&SchemaArgs {
				endianness: Endianness::Little,
				custom_metadata: None,
				fields: Some(fields),
				features: Some(features),
			},
		)
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
	pub(crate) name: String,
	pub(crate) nullable: bool,
	pub(crate) type_: DataType,
	pub(crate) children: Vec<Field>,
}

impl Field {
	pub(crate) fn create_field<'fbb>(
		&self,
		builder: &mut FlatBufferBuilder<'fbb>,
	) -> WIPOffset<FbField<'fbb>> {
		let name = builder.create_string(&self.name);
		let data_type = self.type_.create_fb_type(builder);

		let children: Vec<_> = self
			.children
			.iter()
			.map(|c| c.create_field(builder))
			.collect();
		let children = builder.create_vector(&children);

		FbField::create(
			builder,
			&FieldArgs {
				name: Some(name),
				nullable: self.nullable,
				type_type: self.type_.fb_type_disc(),
				type_: Some(data_type),
				dictionary: None,
				children: Some(children),
				custom_metadata: None,
			},
		)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataType {
	Null,
	Int {
		bit_width: i32,
		is_signed: bool,
	},
	FloatingPoint {
		precision: Precision,
	},
	Binary,
	Utf8,
	Bool,
	Decimal {
		precision: i32,
		scale: i32,
		bit_width: i32,
	},
	Date {
		unit: DateUnit,
	},
	Time {
		unit: TimeUnit,
		bit_width: i32,
	},
	Timestamp {
		unit: TimeUnit,
		timezone: Option<Box<str>>,
	},
	Interval {
		unit: IntervalUnit,
	},
	List,
	FixedSizeBinary {
		byte_width: i32,
	},
	FixedSizeList {
		list_size: i32,
	},
	Map {
		keys_sorted: bool,
	},
	Duration {
		unit: TimeUnit,
	},
	LargeBinary,
	LargeUtf8,
	LargeList,
	RunEndEncoded,
	BinaryView,
	Utf8View,
	ListView,
	LargeListView,
}

impl DataType {
	pub(crate) fn fb_type_disc(&self) -> Type {
		match self {
			DataType::Null => Type::Null,
			DataType::Int { .. } => Type::Int,
			DataType::FloatingPoint { .. } => Type::FloatingPoint,
			DataType::Binary => Type::Binary,
			DataType::Utf8 => Type::Utf8,
			DataType::Bool => Type::Bool,
			DataType::Decimal { .. } => Type::Decimal,
			DataType::Date { .. } => Type::Date,
			DataType::Time { .. } => Type::Time,
			DataType::Timestamp { .. } => Type::Timestamp,
			DataType::Interval { .. } => Type::Interval,
			DataType::List => Type::List,
			DataType::FixedSizeBinary { .. } => {
				Type::FixedSizeBinary
			}
			DataType::FixedSizeList { .. } => Type::FixedSizeList,
			DataType::Map { .. } => Type::Map,
			DataType::Duration { .. } => Type::Duration,
			DataType::LargeBinary => Type::LargeBinary,
			DataType::LargeUtf8 => Type::LargeUtf8,
			DataType::LargeList => Type::LargeList,
			DataType::RunEndEncoded => Type::RunEndEncoded,
			DataType::BinaryView => Type::BinaryView,
			DataType::Utf8View => Type::Utf8View,
			DataType::ListView => Type::ListView,
			DataType::LargeListView => Type::LargeListView,
		}
	}

	pub(crate) fn create_fb_type(
		&self,
		builder: &mut FlatBufferBuilder<'_>,
	) -> WIPOffset<UnionWIPOffset> {
		match self {
			DataType::Null => Null::create(builder, &NullArgs {})
				.as_union_value(),
			DataType::Int {
				bit_width,
				is_signed,
			} => Int::create(
				builder,
				&IntArgs {
					bitWidth: *bit_width,
					is_signed: *is_signed,
				},
			)
			.as_union_value(),
			DataType::FloatingPoint { precision } => {
				FloatingPoint::create(
					builder,
					&FloatingPointArgs {
						precision: *precision,
					},
				)
				.as_union_value()
			}
			DataType::Binary => {
				Binary::create(builder, &BinaryArgs {})
					.as_union_value()
			}
			DataType::Utf8 => Utf8::create(builder, &Utf8Args {})
				.as_union_value(),
			DataType::Bool => Bool::create(builder, &BoolArgs {})
				.as_union_value(),
			DataType::Decimal {
				precision,
				scale,
				bit_width,
			} => Decimal::create(
				builder,
				&DecimalArgs {
					precision: *precision,
					scale: *scale,
					bitWidth: *bit_width,
				},
			)
			.as_union_value(),
			DataType::Date { unit } => {
				Date::create(builder, &DateArgs { unit: *unit })
					.as_union_value()
			}
			DataType::Time { unit, bit_width } => Time::create(
				builder,
				&TimeArgs {
					unit: *unit,
					bitWidth: *bit_width,
				},
			)
			.as_union_value(),
			DataType::Timestamp { unit, timezone } => {
				let tz_offset = timezone
					.as_ref()
					.map(|tz| builder.create_string(tz));
				Timestamp::create(
					builder,
					&TimestampArgs {
						unit: *unit,
						timezone: tz_offset,
					},
				)
				.as_union_value()
			}
			DataType::Interval { unit } => Interval::create(
				builder,
				&IntervalArgs { unit: *unit },
			)
			.as_union_value(),
			DataType::List => List::create(builder, &ListArgs {})
				.as_union_value(),
			DataType::FixedSizeBinary { byte_width } => {
				FixedSizeBinary::create(
					builder,
					&FixedSizeBinaryArgs {
						byteWidth: *byte_width,
					},
				)
				.as_union_value()
			}
			DataType::FixedSizeList { list_size } => {
				FixedSizeList::create(
					builder,
					&FixedSizeListArgs {
						listSize: *list_size,
					},
				)
				.as_union_value()
			}
			DataType::Map { keys_sorted } => Map::create(
				builder,
				&MapArgs {
					keysSorted: *keys_sorted,
				},
			)
			.as_union_value(),
			DataType::Duration { unit } => Duration::create(
				builder,
				&DurationArgs { unit: *unit },
			)
			.as_union_value(),
			DataType::LargeBinary => LargeBinary::create(
				builder,
				&LargeBinaryArgs {},
			)
			.as_union_value(),
			DataType::LargeUtf8 => {
				LargeUtf8::create(builder, &LargeUtf8Args {})
					.as_union_value()
			}
			DataType::LargeList => {
				LargeList::create(builder, &LargeListArgs {})
					.as_union_value()
			}
			DataType::RunEndEncoded => RunEndEncoded::create(
				builder,
				&RunEndEncodedArgs {},
			)
			.as_union_value(),
			DataType::BinaryView => {
				BinaryView::create(builder, &BinaryViewArgs {})
					.as_union_value()
			}
			DataType::Utf8View => {
				Utf8View::create(builder, &Utf8ViewArgs {})
					.as_union_value()
			}
			DataType::ListView => {
				ListView::create(builder, &ListViewArgs {})
					.as_union_value()
			}
			DataType::LargeListView => LargeListView::create(
				builder,
				&LargeListViewArgs {},
			)
			.as_union_value(),
		}
	}
}
