use arbtest::arbtest;
use picoarrow::array::{
	Array, ArrayPrimitive, ArrayStruct, NonNullable, Nullable,
};

type Fields = (
	ArrayPrimitive<i32, NonNullable>,
	ArrayPrimitive<i64, NonNullable>,
);

#[test]
fn simulate_struct() {
	arbtest(|u| {
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected_ids: Vec<i32> = Vec::new();
		let mut expected_scores: Vec<i64> = Vec::new();
		let mut arr: ArrayStruct<Fields, NonNullable> =
			ArrayStruct::new(
				vec!["id".into(), "score".into()],
				(ArrayPrimitive::new(), ArrayPrimitive::new()),
			);

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected_ids.clear();
				expected_scores.clear();
				arr.clear();
			} else {
				let id = u.arbitrary::<i32>()?;
				let score = u.arbitrary::<i64>()?;
				arr.push(|fields| {
					fields.0.push(id);
					fields.1.push(score);
				})
				.unwrap();
				expected_ids.push(id);
				expected_scores.push(score);
			}

			assert_eq!(arr.len(), expected_ids.len());
			let fields = arr.fields();
			for (i, &id) in expected_ids.iter().enumerate() {
				assert_eq!(*fields.0.get(i), id);
			}
			for (i, &score) in expected_scores.iter().enumerate() {
				assert_eq!(*fields.1.get(i), score);
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(20));
}

#[test]
fn simulate_struct_nullable() {
	arbtest(|u| {
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<Option<(i32, i64)>> = Vec::new();
		let mut arr: ArrayStruct<Fields, Nullable> = ArrayStruct::new(
			vec!["id".into(), "score".into()],
			(ArrayPrimitive::new(), ArrayPrimitive::new()),
		);

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else if u.ratio(1, 3)? {
				arr.push_null(|fields| {
					fields.0.push(0);
					fields.1.push(0);
				})
				.unwrap();
				expected.push(None);
			} else {
				let id = u.arbitrary::<i32>()?;
				let score = u.arbitrary::<i64>()?;
				arr.push(|fields| {
					fields.0.push(id);
					fields.1.push(score);
				})
				.unwrap();
				expected.push(Some((id, score)));
			}

			assert_eq!(arr.len(), expected.len());
			for (i, v) in expected.iter().enumerate() {
				match v {
					Some((id, score)) => {
						assert!(!arr.is_null(i));
						let fields = arr.fields();
						assert_eq!(
							*fields.0.get(i),
							*id
						);
						assert_eq!(
							*fields.1.get(i),
							*score
						);
					}
					None => {
						assert!(arr.is_null(i));
					}
				}
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(20));
}

#[test]
fn struct_push_wrong_length() {
	let mut arr: ArrayStruct<Fields, NonNullable> = ArrayStruct::new(
		vec!["id".into(), "score".into()],
		(ArrayPrimitive::new(), ArrayPrimitive::new()),
	);

	let result = arr.push(|fields| {
		fields.0.push(1);
	});
	assert!(result.is_err());
	assert_eq!(arr.len(), 0);
}
