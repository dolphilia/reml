use reml_runtime_ffi::core_prelude::iter::Iter;

#[test]
fn from_list_roundtrip() {
    let iter = Iter::from_list(vec![1, 2, 3]);
    assert_eq!(iter.collect_vec(), vec![1, 2, 3]);
}

#[test]
fn from_result_passthrough() {
    assert_eq!(
        Iter::<i32>::from_result::<&str>(Ok(5)).collect_vec(),
        vec![5]
    );
    assert!(Iter::<i32>::from_result::<&str>(Err("boom"))
        .collect_vec()
        .is_empty());
}

#[test]
fn from_fn_counter() {
    let mut value = 0;
    let iter = Iter::from_fn(move || {
        if value >= 3 {
            None
        } else {
            let current = value;
            value += 1;
            Some(current)
        }
    });
    assert_eq!(iter.collect_vec(), vec![0, 1, 2]);
}

#[test]
fn range_basic() {
    let iter = Iter::range(0, 5, 1);
    assert_eq!(iter.collect_vec(), vec![0, 1, 2, 3, 4, 5]);
}

#[test]
fn range_descending() {
    let iter = Iter::range(5, 1, -2);
    assert_eq!(iter.collect_vec(), vec![5, 3, 1]);
}

#[test]
fn repeat_take() {
    let iter = Iter::repeat("hi".to_string());
    let mut values = Vec::new();
    for _ in 0..3 {
        values.push(iter.next().unwrap());
    }
    assert_eq!(
        values,
        vec!["hi".to_string(), "hi".to_string(), "hi".to_string()]
    );
}

#[test]
fn unfold_fibonacci_sequence() {
    let iter = Iter::unfold(
        (0, 1),
        |(a, b)| {
            if a > 21 {
                None
            } else {
                Some((a, (b, a + b)))
            }
        },
    );
    assert_eq!(iter.collect_vec(), vec![0, 1, 1, 2, 3, 5, 8, 13, 21]);
}

#[test]
fn try_unfold_terminates_on_error() {
    struct End;
    let iter = Iter::try_unfold::<_, End, _>(0, |state| {
        if state >= 3 {
            Err(End)
        } else {
            Ok(Some((state * 2, state + 1)))
        }
    });
    assert_eq!(iter.collect_vec(), vec![0, 2, 4]);
}
