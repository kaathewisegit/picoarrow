use arbitrary::{Arbitrary, Result, Unstructured};
use arbtest::arbtest;
use pastey::paste;

use core::{borrow::Borrow, fmt::Debug};

#[cfg(feature = "half")]
use half::f16;
use picoarrow::array::{
	Array, ArrayPrimitive, NonNullable, Nullable, Primitive,
};

macro_rules! create_test {
	($name:ident) => {
		create_test!(u8, $name);
		create_test!(u16, $name);
		create_test!(u32, $name);
		create_test!(u64, $name);
		create_test!(i8, $name);
		create_test!(i16, $name);
		create_test!(i32, $name);
		create_test!(i64, $name);
		create_test!(f32, $name);
		create_test!(f64, $name);
		#[cfg(feature = "half")]
		create_test!(f16, $name);
	};

	($ty:ty, $name:ident) => {
		paste! {
			#[test]
			fn [<$name _ $ty>] () {
				arbtest(|u| $name::<$ty>(u))
				.size_min(2u32.pow(15))
				.size_max(2u32.pow(20));
			}
		}
	};
}

fn from_vec<'a, T: Primitive + Arbitrary<'a> + PartialEq + Debug>(
	u: &mut Unstructured<'a>,
) -> Result<()> {
	let len = u.int_in_range(0..=5_000)?;
	let mut values = Vec::<T>::new();
	for _ in 0..len {
		values.push(u.arbitrary()?);
	}

	let arr: ArrayPrimitive<T, Nullable> =
		ArrayPrimitive::from_vec(values.clone());

	assert_eq!(arr.len(), values.len());

	for (i, val) in values.iter().enumerate() {
		let arr_val = arr.get(i).unwrap();
		assert_eq!(
			arr_val.to_le_bytes().borrow(),
			val.to_le_bytes().borrow()
		);
	}
	Ok(())
}
create_test!(from_vec);

#[test]
fn from_vec_enumerate_lengths() {
	let mut v = Vec::new();

	for i in 0..2000 {
		let arr: ArrayPrimitive<i32, Nullable> =
			ArrayPrimitive::from_vec(v.clone());

		for (i, val) in v.iter().enumerate() {
			let arr_val = arr.get(i).unwrap();
			assert_eq!(arr_val.to_le_bytes(), val.to_le_bytes());
		}
		v.push(i);
	}
}

fn simulate<'a, T: Primitive + Arbitrary<'a> + PartialEq + Debug>(
	u: &mut Unstructured<'a>,
) -> Result<()> {
	let steps = u.int_in_range(0..=5_000)?;
	let mut expected = Vec::new();
	let mut arr: ArrayPrimitive<T, NonNullable> = ArrayPrimitive::new();

	for _ in 0..steps {
		if u.ratio(1, 100)? {
			expected.clear();
			arr.clear();
		} else {
			let val: T = u.arbitrary()?;
			expected.push(val);
			arr.push(val);
		}

		assert_eq!(arr.len(), expected.len());
		for (i, v) in expected.iter().enumerate() {
			assert_eq!(arr.get(i).to_le_bytes(), v.to_le_bytes());
		}
	}
	Ok(())
}
create_test!(simulate);

fn simulate_nullable<'a, T: Primitive + Arbitrary<'a> + PartialEq + Debug>(
	u: &mut Unstructured<'a>,
) -> Result<()> {
	let steps = u.int_in_range(0..=5_000)?;
	let mut expected: Vec<Option<T>> = Vec::new();
	let mut arr: ArrayPrimitive<T, Nullable> = ArrayPrimitive::new();

	for _ in 0..steps {
		if u.ratio(1, 100)? {
			expected.clear();
			arr.clear();
		} else if u.ratio(1, 3)? {
			expected.push(None);
			arr.push_null();
		} else {
			let val: T = u.arbitrary()?;
			expected.push(Some(val));
			arr.push_some(val);
		}

		assert_eq!(arr.len(), expected.len());
		assert_eq!(
			arr.null_count(),
			expected.iter().filter(|v| v.is_none()).count()
		);
		for (i, v) in expected.iter().enumerate() {
			match v {
				Some(val) => {
					let arr_val = arr.get(i).unwrap();
					assert_eq!(
						arr_val.to_le_bytes(),
						val.to_le_bytes()
					);
				}
				None => {
					assert!(arr.is_null(i));
					assert!(arr.get(i).is_none());
				}
			}
		}
	}
	Ok(())
}
create_test!(simulate_nullable);

fn extend_iter<'a, T: Primitive + Arbitrary<'a> + PartialEq + Debug>(
	u: &mut Unstructured<'a>,
) -> Result<()> {
	let steps = u.int_in_range(0..=500)?;
	let mut expected: Vec<T> = Vec::new();
	let mut arr: ArrayPrimitive<T, NonNullable> = ArrayPrimitive::new();

	for _ in 0..steps {
		let chunk_len = u.int_in_range(0..=512)?;
		let mut chunk = Vec::with_capacity(chunk_len);
		for _ in 0..chunk_len {
			chunk.push(u.arbitrary()?);
		}
		expected.extend(chunk.iter().copied());
		arr.extend(chunk.iter().copied());

		assert_eq!(arr.len(), expected.len());
	}

	for (i, v) in expected.iter().enumerate() {
		assert_eq!(arr.get(i).to_le_bytes(), v.to_le_bytes());
	}
	Ok(())
}
create_test!(extend_iter);

#[test]
fn extend_emtpy() {
	let mut arr = ArrayPrimitive::<u32, Nullable>::new();
	arr.extend_from_slice(&[]);
}

fn extend_iter_nullable<
	'a,
	T: Primitive + Arbitrary<'a> + PartialEq + Debug,
>(
	u: &mut Unstructured<'a>,
) -> Result<()> {
	let steps = u.int_in_range(0..=500)?;
	let mut expected: Vec<Option<T>> = Vec::new();
	let mut arr: ArrayPrimitive<T, Nullable> = ArrayPrimitive::new();

	for _ in 0..steps {
		let chunk_len = u.int_in_range(0..=512)?;
		let mut chunk: Vec<Option<T>> = Vec::with_capacity(chunk_len);
		for _ in 0..chunk_len {
			if u.ratio(1, 3)? {
				chunk.push(None);
			} else {
				chunk.push(Some(u.arbitrary()?));
			}
		}
		expected.extend(chunk.iter().copied());
		arr.extend(chunk.iter().copied());

		assert_eq!(arr.len(), expected.len());
		assert_eq!(
			arr.null_count(),
			expected.iter().filter(|v| v.is_none()).count()
		);
	}

	for (i, v) in expected.iter().enumerate() {
		match v {
			Some(val) => {
				assert!(!arr.is_null(i));
				assert_eq!(
					arr.get(i).unwrap().to_le_bytes(),
					val.to_le_bytes()
				);
			}
			None => {
				assert!(arr.is_null(i));
				assert!(arr.get(i).is_none());
			}
		}
	}
	Ok(())
}
create_test!(extend_iter_nullable);
