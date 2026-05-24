use arbtest::arbtest;
#[cfg(feature = "half")]
use half::f16;
use picoarrow::array::{Array, ArrayPrimitive, NonNullable, Nullable};

macro_rules! test_from_vec {
	($name:ident, $ty:ty) => {
		#[test]
		fn $name() {
			arbtest(|u| {
				let len = u.int_in_range(0..=5_000)?;
				let mut values = Vec::<$ty>::new();
				for _ in 0..len {
					values.push(u.arbitrary()?);
				}

				let arr: ArrayPrimitive<$ty, Nullable> =
					ArrayPrimitive::from_vec(
						values.clone(),
					);

				assert_eq!(arr.len(), values.len());

				for (i, val) in values.iter().enumerate() {
					let arr_val = arr.get(i).unwrap();
					assert_eq!(
						arr_val.to_le_bytes(),
						val.to_le_bytes()
					);
				}
				Ok(())
			})
			.size_min(2u32.pow(15))
			.size_max(2u32.pow(20));
		}
	};
}

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

test_from_vec!(u8, u8);
test_from_vec!(u16, u16);
test_from_vec!(u32, u32);
test_from_vec!(u64, u64);
test_from_vec!(i8, i8);
test_from_vec!(i16, i16);
test_from_vec!(i32, i32);
test_from_vec!(i64, i64);
test_from_vec!(f32, f32);
test_from_vec!(f64, f64);
#[cfg(feature = "half")]
test_from_vec!(f16, f16);

macro_rules! test_simulate {
	($name:ident, $ty:ty) => {
		#[test]
		fn $name() {
			arbtest(|u| {
				let steps = u.int_in_range(0..=5_000)?;
				let mut expected: Vec<$ty> = Vec::new();
				let mut arr: ArrayPrimitive<$ty, NonNullable> =
					ArrayPrimitive::new();

				for _ in 0..steps {
					if u.ratio(1, 100)? {
						expected.clear();
						arr.clear();
					} else {
						let val: $ty = u.arbitrary()?;
						expected.push(val);
						arr.push(val);
					}

					assert_eq!(arr.len(), expected.len());
					for (i, v) in
						expected.iter().enumerate()
					{
						assert_eq!(
							arr.get(i)
								.to_le_bytes(),
							v.to_le_bytes()
						);
					}
				}
				Ok(())
			})
			.size_min(2u32.pow(15))
			.size_max(2u32.pow(20));
		}
	};
}

test_simulate!(simulate_u8, u8);
test_simulate!(simulate_u16, u16);
test_simulate!(simulate_u32, u32);
test_simulate!(simulate_u64, u64);
test_simulate!(simulate_i8, i8);
test_simulate!(simulate_i16, i16);
test_simulate!(simulate_i32, i32);
test_simulate!(simulate_i64, i64);
test_simulate!(simulate_f32, f32);
test_simulate!(simulate_f64, f64);
#[cfg(feature = "half")]
test_simulate!(simulate_f16, f16);

macro_rules! test_simulate_nullable {
	($name:ident, $ty:ty) => {
		#[test]
		fn $name() {
			arbtest(|u| {
				let steps = u.int_in_range(0..=5_000)?;
				let mut expected: Vec<Option<$ty>> = Vec::new();
				let mut arr: ArrayPrimitive<$ty, Nullable> =
					ArrayPrimitive::new();

				for _ in 0..steps {
					if u.ratio(1, 100)? {
						expected.clear();
						arr.clear();
					} else if u.ratio(1, 3)? {
						expected.push(None);
						arr.push_null();
					} else {
						let val: $ty = u.arbitrary()?;
						expected.push(Some(val));
						arr.push(val);
					}

					assert_eq!(arr.len(), expected.len());
					assert_eq!(
						arr.null_count(),
						expected.iter().filter(|v| v.is_none()).count()
					);
					for (i, v) in
						expected.iter().enumerate()
					{
						match v {
							Some(val) => {
								let arr_val =
									arr.get(i).unwrap();
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
			})
			.size_min(2u32.pow(16));
		}
	};
}

test_simulate_nullable!(simulate_nullable_u8, u8);
test_simulate_nullable!(simulate_nullable_u16, u16);
test_simulate_nullable!(simulate_nullable_u32, u32);
test_simulate_nullable!(simulate_nullable_u64, u64);
test_simulate_nullable!(simulate_nullable_i8, i8);
test_simulate_nullable!(simulate_nullable_i16, i16);
test_simulate_nullable!(simulate_nullable_i32, i32);
test_simulate_nullable!(simulate_nullable_i64, i64);
test_simulate_nullable!(simulate_nullable_f32, f32);
test_simulate_nullable!(simulate_nullable_f64, f64);
#[cfg(feature = "half")]
test_simulate_nullable!(simulate_nullable_f16, f16);
