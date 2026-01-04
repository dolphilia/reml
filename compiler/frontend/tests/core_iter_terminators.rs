use reml_runtime_ffi::core_prelude::iter::Iter;

#[test]
fn iter_fold_accumulates_values() {
    let sum = Iter::from_list(vec![1, 2, 3, 4]).fold(0, |acc, value| acc + value);
    assert_eq!(sum, 10);

    let empty_sum = Iter::<i32>::empty().fold(42, |acc, _| acc + 1);
    assert_eq!(empty_sum, 42);
}

#[test]
fn iter_reduce_merges_elements() {
    let product = Iter::from_list(vec![2, 3, 4]).reduce(|acc, value| acc * value);
    assert_eq!(product, Some(24));

    let none = Iter::<i32>::empty().reduce(|acc, value| acc + value);
    assert!(none.is_none());
}

#[test]
fn iter_all_any_find_behave_like_std_iterator() {
    let new_iter = || Iter::from_list(vec![1, 2, 3, 4]);
    assert!(new_iter().all(|value| *value > 0));
    assert!(!new_iter().all(|value| *value < 4));
    assert!(new_iter().any(|value| *value == 3));
    assert!(!new_iter().any(|value| *value == 99));

    let found = new_iter().find(|value| *value % 2 == 0);
    assert_eq!(found, Some(2));
    assert!(new_iter().find(|value| *value == 10).is_none());
}

#[test]
fn iter_try_fold_short_circuits_on_error() {
    let iter = Iter::from_list(vec![1, 2, 3, 4]);
    let result = iter.try_fold(0, |acc, value| {
        if value == 3 {
            Err("boom")
        } else {
            Ok(acc + value)
        }
    });
    assert_eq!(result, Err("boom"));
}

#[test]
fn iter_try_fold_success() {
    let iter = Iter::from_list(vec![1, 2, 3]);
    let result = iter.try_fold(String::new(), |mut acc, value| -> Result<_, ()> {
        acc.push_str(&value.to_string());
        Ok(acc)
    });
    assert_eq!(result.as_deref(), Ok("123"));
}
