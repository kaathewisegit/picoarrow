use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use crate::array::Array;
use crate::fb::{
	Binary, BinaryArgs, BinaryView, BinaryViewArgs, Bool, BoolArgs, Date,
	DateArgs, DateUnit, Decimal, DecimalArgs, Duration, DurationArgs,
	Endianness, Feature, Field as FbField, FieldArgs, FixedSizeBinary,
	FixedSizeBinaryArgs, FixedSizeList, FixedSizeListArgs, FloatingPoint,
	FloatingPointArgs, Int, IntArgs, Interval, IntervalArgs, IntervalUnit,
	KeyValue, KeyValueArgs, LargeBinary, LargeBinaryArgs, LargeList,
	LargeListArgs, LargeListView, LargeListViewArgs, LargeUtf8,
	LargeUtf8Args, List, ListArgs, ListView, ListViewArgs, Map, MapArgs,
	Null, NullArgs, Precision, RunEndEncoded, RunEndEncodedArgs,
	Schema as FbSchema, SchemaArgs, Time, TimeArgs, TimeUnit, Timestamp,
	TimestampArgs, Type, Utf8, Utf8Args, Utf8View, Utf8ViewArgs,
};

/// Describes the types of a collection of arrays which can be serialized via
/// Arrow IPC
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
	pub fields: Vec<Field>,
	pub custom_metadata: Vec<(String, String)>,
}

impl Schema {
	pub fn from_fields<I>(fields: I) -> Self
	where
		I: IntoIterator<Item = Field>,
	{
		Self {
			fields: fields.into_iter().collect(),
			custom_metadata: Vec::new(),
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

		Self {
			fields,
			custom_metadata: Vec::new(),
		}
	}

	#[allow(dead_code)]
	pub(crate) fn deserialize(fb: &FbSchema<'_>) -> Self {
		let fields = fb
			.fields()
			.map(|f| f.iter().map(|f| Field::from_fb(&f)).collect())
			.unwrap_or_default();

		let custom_metadata = fb
			.custom_metadata()
			.map(|kv| {
				kv.iter()
					.filter_map(|kv| {
						let key = kv.key()?;
						let value = kv
							.value()
							.unwrap_or_default();
						Some((
							key.to_owned(),
							value.to_owned(),
						))
					})
					.collect()
			})
			.unwrap_or_default();

		Self {
			fields,
			custom_metadata,
		}
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

		let custom_metadata = if self.custom_metadata.is_empty() {
			None
		} else {
			let kv_offsets: Vec<_> = self
				.custom_metadata
				.iter()
				.map(|(key, value)| {
					let key = builder.create_string(key);
					let value =
						builder.create_string(value);
					KeyValue::create(
						builder,
						&KeyValueArgs {
							key: Some(key),
							value: Some(value),
						},
					)
				})
				.collect();
			Some(builder.create_vector(&kv_offsets))
		};

		FbSchema::create(
			builder,
			&SchemaArgs {
				endianness: Endianness::Little,
				custom_metadata,
				fields: Some(fields),
				features: Some(features),
			},
		)
	}
}

/// A named column in a record/row batch
///
/// Fields can have children, which can also be named.  But `picoarrow` only
/// supports lists, so nested arrays will always have a name of `item`.
#[derive(Debug, Clone)]
pub struct Field {
	pub(crate) name: String,
	pub(crate) nullable: bool,
	pub(crate) type_: DataType,
	pub(crate) children: Vec<Field>,
}

impl PartialEq for Field {
	fn eq(&self, other: &Self) -> bool {
		self.type_ == other.type_ && self.children == other.children
	}
}

impl Field {
	#[allow(dead_code)]
	pub(crate) fn from_fb(field: &FbField<'_>) -> Self {
		Self {
			name: field.name().unwrap_or_default().to_owned(),
			nullable: field.nullable(),
			type_: DataType::from_fb(field).expect("unimlemented"),
			children: field
				.children()
				.map(|c| {
					c.iter().map(|c| Field::from_fb(&c))
						.collect()
				})
				.unwrap_or_default(),
		}
	}

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

	#[allow(dead_code)]
	pub(crate) fn from_fb(field: &FbField<'_>) -> Option<Self> {
		match field.type_type() {
			Type::NONE => None,
			Type::Null => Some(DataType::Null),
			Type::Int => {
				field.type__as_int().map(|int| DataType::Int {
					bit_width: int.bitWidth(),
					is_signed: int.is_signed(),
				})
			}
			Type::FloatingPoint => field
				.type__as_floating_point()
				.map(|fp| DataType::FloatingPoint {
					precision: fp.precision(),
				}),
			Type::Binary => Some(DataType::Binary),
			Type::Utf8 => Some(DataType::Utf8),
			Type::Bool => Some(DataType::Bool),
			Type::Decimal => field.type__as_decimal().map(|d| {
				DataType::Decimal {
					precision: d.precision(),
					scale: d.scale(),
					bit_width: d.bitWidth(),
				}
			}),
			Type::Date => field
				.type__as_date()
				.map(|d| DataType::Date { unit: d.unit() }),
			Type::Time => {
				field.type__as_time().map(|t| DataType::Time {
					unit: t.unit(),
					bit_width: t.bitWidth(),
				})
			}
			Type::Timestamp => {
				field.type__as_timestamp().map(|ts| {
					DataType::Timestamp {
						unit: ts.unit(),
						timezone: ts.timezone().map(
							|tz| {
								tz.to_owned()
									.into_boxed_str(
									)
							},
						),
					}
				})
			}
			Type::Interval => field
				.type__as_interval()
				.map(|i| DataType::Interval { unit: i.unit() }),
			Type::List => Some(DataType::List),
			Type::FixedSizeBinary => field
				.type__as_fixed_size_binary()
				.map(|fsb| DataType::FixedSizeBinary {
					byte_width: fsb.byteWidth(),
				}),
			Type::FixedSizeList => field
				.type__as_fixed_size_list()
				.map(|fsl| DataType::FixedSizeList {
					list_size: fsl.listSize(),
				}),
			Type::Map => {
				field.type__as_map().map(|m| DataType::Map {
					keys_sorted: m.keysSorted(),
				})
			}
			Type::Duration => field
				.type__as_duration()
				.map(|d| DataType::Duration { unit: d.unit() }),
			Type::LargeBinary => Some(DataType::LargeBinary),
			Type::LargeUtf8 => Some(DataType::LargeUtf8),
			Type::LargeList => Some(DataType::LargeList),
			Type::RunEndEncoded => Some(DataType::RunEndEncoded),
			Type::BinaryView => Some(DataType::BinaryView),
			Type::Utf8View => Some(DataType::Utf8View),
			Type::ListView => Some(DataType::ListView),
			Type::LargeListView => Some(DataType::LargeListView),
			_ => None,
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

#[cfg(test)]
mod tests {
	use super::*;

	fn roundtrip_schema(schema: &Schema) {
		let mut builder = FlatBufferBuilder::new();
		let offset = schema.serialize(&mut builder);
		builder.finish(offset, None);
		let bytes = builder.finished_data();

		let fb = flatbuffers::root::<FbSchema<'_>>(bytes).unwrap();
		let deserialized = Schema::deserialize(&fb);

		assert_eq!(schema, &deserialized);
	}

	#[test]
	fn empty_schema() {
		let schema = Schema {
			fields: vec![],
			custom_metadata: vec![],
		};
		roundtrip_schema(&schema);
	}

	#[test]
	fn schema_with_metadata() {
		let schema = Schema {
			fields: vec![],
			custom_metadata: vec![
				("key1".into(), "value1".into()),
				("key2".into(), "value2".into()),
			],
		};
		roundtrip_schema(&schema);
	}

	#[test]
	fn schema_with_fields() {
		let schema = Schema {
			fields: vec![
				Field {
					name: "col1".into(),
					nullable: false,
					type_: DataType::Int {
						bit_width: 32,
						is_signed: true,
					},
					children: vec![],
				},
				Field {
					name: "col2".into(),
					nullable: true,
					type_: DataType::Utf8,
					children: vec![],
				},
			],
			custom_metadata: vec![],
		};
		roundtrip_schema(&schema);
	}

	#[test]
	fn all_data_types() {
		let schema = Schema {
			fields: vec![
				Field {
					name: "null".into(),
					nullable: true,
					type_: DataType::Null,
					children: vec![],
				},
				Field {
					name: "int8".into(),
					nullable: false,
					type_: DataType::Int {
						bit_width: 8,
						is_signed: true,
					},
					children: vec![],
				},
				Field {
					name: "uint64".into(),
					nullable: false,
					type_: DataType::Int {
						bit_width: 64,
						is_signed: false,
					},
					children: vec![],
				},
				Field {
					name: "float32".into(),
					nullable: false,
					type_: DataType::FloatingPoint {
						precision: Precision::SINGLE,
					},
					children: vec![],
				},
				Field {
					name: "float64".into(),
					nullable: false,
					type_: DataType::FloatingPoint {
						precision: Precision::DOUBLE,
					},
					children: vec![],
				},
				Field {
					name: "bool".into(),
					nullable: false,
					type_: DataType::Bool,
					children: vec![],
				},
				Field {
					name: "binary".into(),
					nullable: false,
					type_: DataType::Binary,
					children: vec![],
				},
				Field {
					name: "utf8".into(),
					nullable: false,
					type_: DataType::Utf8,
					children: vec![],
				},
				Field {
					name: "decimal".into(),
					nullable: false,
					type_: DataType::Decimal {
						precision: 38,
						scale: 10,
						bit_width: 256,
					},
					children: vec![],
				},
				Field {
					name: "date32".into(),
					nullable: false,
					type_: DataType::Date {
						unit: DateUnit::DAY,
					},
					children: vec![],
				},
				Field {
					name: "date64".into(),
					nullable: false,
					type_: DataType::Date {
						unit: DateUnit::MILLISECOND,
					},
					children: vec![],
				},
				Field {
					name: "time32".into(),
					nullable: false,
					type_: DataType::Time {
						unit: TimeUnit::SECOND,
						bit_width: 32,
					},
					children: vec![],
				},
				Field {
					name: "time64".into(),
					nullable: false,
					type_: DataType::Time {
						unit: TimeUnit::NANOSECOND,
						bit_width: 64,
					},
					children: vec![],
				},
				Field {
					name: "timestamp_no_tz".into(),
					nullable: false,
					type_: DataType::Timestamp {
						unit: TimeUnit::MICROSECOND,
						timezone: None,
					},
					children: vec![],
				},
				Field {
					name: "timestamp_with_tz".into(),
					nullable: false,
					type_: DataType::Timestamp {
						unit: TimeUnit::SECOND,
						timezone: Some("UTC".into()),
					},
					children: vec![],
				},
				Field {
					name: "interval".into(),
					nullable: false,
					type_: DataType::Interval {
						unit: IntervalUnit::YEAR_MONTH,
					},
					children: vec![],
				},
				Field {
					name: "list".into(),
					nullable: false,
					type_: DataType::List,
					children: vec![Field {
						name: "item".into(),
						nullable: false,
						type_: DataType::Int {
							bit_width: 32,
							is_signed: true,
						},
						children: vec![],
					}],
				},
				Field {
					name: "fsb".into(),
					nullable: false,
					type_: DataType::FixedSizeBinary {
						byte_width: 16,
					},
					children: vec![],
				},
				Field {
					name: "fsl".into(),
					nullable: false,
					type_: DataType::FixedSizeList {
						list_size: 3,
					},
					children: vec![Field {
						name: "item".into(),
						nullable: false,
						type_: DataType::Int {
							bit_width: 32,
							is_signed: true,
						},
						children: vec![],
					}],
				},
				Field {
					name: "map".into(),
					nullable: false,
					type_: DataType::Map {
						keys_sorted: true,
					},
					children: vec![],
				},
				Field {
					name: "duration".into(),
					nullable: false,
					type_: DataType::Duration {
						unit: TimeUnit::NANOSECOND,
					},
					children: vec![],
				},
				Field {
					name: "large_binary".into(),
					nullable: false,
					type_: DataType::LargeBinary,
					children: vec![],
				},
				Field {
					name: "large_utf8".into(),
					nullable: false,
					type_: DataType::LargeUtf8,
					children: vec![],
				},
				Field {
					name: "large_list".into(),
					nullable: false,
					type_: DataType::LargeList,
					children: vec![Field {
						name: "item".into(),
						nullable: false,
						type_: DataType::Int {
							bit_width: 64,
							is_signed: true,
						},
						children: vec![],
					}],
				},
				Field {
					name: "ree".into(),
					nullable: false,
					type_: DataType::RunEndEncoded,
					children: vec![],
				},
				Field {
					name: "binary_view".into(),
					nullable: false,
					type_: DataType::BinaryView,
					children: vec![],
				},
				Field {
					name: "utf8_view".into(),
					nullable: false,
					type_: DataType::Utf8View,
					children: vec![],
				},
				Field {
					name: "list_view".into(),
					nullable: false,
					type_: DataType::ListView,
					children: vec![],
				},
				Field {
					name: "large_list_view".into(),
					nullable: false,
					type_: DataType::LargeListView,
					children: vec![],
				},
			],
			custom_metadata: vec![("author".into(), "test".into())],
		};
		roundtrip_schema(&schema);
	}
}
