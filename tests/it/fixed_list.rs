use arbtest::arbtest;
use picoarrow::array::{
	Array, ArrayFixedSizeList, ArrayPrimitive, NonNullable, Nullable,
};

#[test]
fn simulate_fixed_list() {
	arbtest(|u| {
		let size = u.int_in_range(1..=8)?;
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<Vec<i32>> = Vec::new();
		let mut arr: ArrayFixedSizeList<
			ArrayPrimitive<i32, NonNullable>,
			NonNullable,
		> = ArrayFixedSizeList::new(ArrayPrimitive::new(), size);

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else {
				let mut vals =
					Vec::with_capacity(size as usize);
				for _ in 0..size {
					vals.push(u.arbitrary::<i32>()?);
				}
				arr.push(|child| {
					for &v in &vals {
						child.push(v);
					}
				})
				.unwrap();
				expected.push(vals);
			}

			assert_eq!(arr.len(), expected.len());
			let child = arr.child();
			for (i, vals) in expected.iter().enumerate() {
				for (j, &v) in vals.iter().enumerate() {
					assert_eq!(
						*child.get(
							i * size as usize + j
						),
						v
					);
				}
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(20));
}

#[test]
fn simulate_fixed_list_nullable() {
	arbtest(|u| {
		let size = u.int_in_range(1..=8)?;
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<Option<Vec<i32>>> = Vec::new();
		let mut arr: ArrayFixedSizeList<
			ArrayPrimitive<i32, NonNullable>,
			Nullable,
		> = ArrayFixedSizeList::new(ArrayPrimitive::new(), size);

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else if u.ratio(1, 3)? {
				expected.push(None);
				arr.push_null(|child| {
					for _ in 0..size {
						child.push(0);
					}
				})
				.unwrap();
			} else {
				let mut vals =
					Vec::with_capacity(size as usize);
				for _ in 0..size {
					vals.push(u.arbitrary::<i32>()?);
				}
				arr.push(|child| {
					for &v in &vals {
						child.push(v);
					}
				})
				.unwrap();
				expected.push(Some(vals));
			}

			assert_eq!(arr.len(), expected.len());
			let mut child_idx = 0;
			for (i, v) in expected.iter().enumerate() {
				match v {
					Some(vals) => {
						assert!(!arr.is_null(i));
						let child = arr.child();
						for (j, &val) in
							vals.iter().enumerate()
						{
							assert_eq!(
								*child.get(child_idx + j),
								val
							);
						}
						child_idx += vals.len();
					}
					None => {
						assert!(arr.is_null(i));
						child_idx += size as usize;
					}
				}
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(20));
}
