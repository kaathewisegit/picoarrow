#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetadataVersion {
	V1,
	V2,
	V3,
	V4,
	V5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i16)]
pub enum Precision {
	Half,
	Single,
	Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i16)]
pub enum DateUnit {
	Day,
	Millisecond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i16)]
pub enum TimeUnit {
	Second,
	Millisecond,
	Microsecond,
	Nanosecond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i16)]
pub enum IntervalUnit {
	YearMonth,
	DayTime,
	MonthDayNano,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i16)]
pub enum UnionMode {
	Sparse,
	Dense,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
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
	Struct,
	Union {
		mode: UnionMode,
		type_ids: Box<[i32]>,
	},
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

impl Type {
	/// Number of buffers an array of this type needs
	///
	/// Doesn't include the validity buffer.
	pub fn num_buffers(&self) -> usize {
		match self {
			Type::Null => 0,

			Type::Int { .. }
			| Type::FloatingPoint { .. }
			| Type::Bool => 1,

			Type::List | Type::LargeList => 2,

			Type::FixedSizeList { .. } => 0,

			Type::ListView | Type::LargeListView => 2,

			Type::Struct => 0,

			Type::Map { .. } => 1,

			Type::Union { mode, .. } => match mode {
				UnionMode::Sparse => 1,
				UnionMode::Dense => 2,
			},

			_ => unimplemented!(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyValue {
	key: String,
	value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
	name: String,
	pub(crate) nullable: bool,
	pub(crate) data_type: Type,
	pub(crate) children: Vec<Field>,
	pub custom_metadata: Vec<KeyValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Schema {
	fields: Vec<Field>,
	pub custom_metadata: Vec<KeyValue>,
}
