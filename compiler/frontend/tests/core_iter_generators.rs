use reml_runtime_ffi::core_prelude::iter::Iter;

fn collect_values<T>(iter: Iter<T>) -> Vec<T> {
    let (core_vec, _) = iter
        .collect_vec()
        .expect("VecCollector should not fail")
        .into_parts();
    core_vec.into_inner()
}

#[test]
fn from_list_roundtrip() {
    let iter = Iter::from_list(vec![1, 2, 3]);
    assert_eq!(collect_values(iter), vec![1, 2, 3]);
}

#[test]
fn from_result_passthrough() {
    assert_eq!(
        collect_values(Iter::<i32>::from_result::<&str>(Ok(5))),
        vec![5]
    );
    assert!(collect_values(Iter::<i32>::from_result::<&str>(Err("boom"))).is_empty());
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
    assert_eq!(collect_values(iter), vec![0, 1, 2]);
}

#[test]
fn range_basic() {
    let iter = Iter::range(0, 5, 1);
    assert_eq!(collect_values(iter), vec![0, 1, 2, 3, 4, 5]);
}

#[test]
fn range_descending() {
    let iter = Iter::range(5, 1, -2);
    assert_eq!(collect_values(iter), vec![5, 3, 1]);
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
    assert_eq!(collect_values(iter), vec![0, 1, 1, 2, 3, 5, 8, 13, 21]);
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
    assert_eq!(collect_values(iter), vec![0, 2, 4]);
}

#[test]
fn empty_iter_reports_pure_stage() {
    let iter = Iter::<i64>::empty();
    assert_eq!(iter.next(), None);
    let stage = iter.stage_snapshot("core_iter_generators::empty_iter_reports_pure_stage");
    assert_eq!(stage.actual, "beta");
    assert_eq!(stage.required.mode, "at_least");
    assert_eq!(stage.required.stage, "beta");
    assert_eq!(stage.kind, "core_iter");
    let effects = iter.effect_labels();
    assert!(!effects.mem);
    assert!(!effects.mutating);
    assert!(!effects.debug);
    assert!(!effects.async_pending);
    assert_eq!(effects.mem_bytes, 0);
    assert_eq!(effects.predicate_calls, 0);
}

#[test]
fn once_iter_emits_single_value_and_stage() {
    let iter = Iter::once(42_i64);
    let stage = iter.stage_snapshot("core_iter_generators::once_iter_emits_single_value_and_stage");
    assert_eq!(stage.actual, "beta");
    assert_eq!(stage.required.mode, "at_least");
    assert_eq!(stage.required.stage, "beta");
    assert_eq!(stage.kind, "core_iter");
    let effects = iter.effect_labels();
    assert!(!effects.mem);
    assert!(!effects.mutating);
    assert!(!effects.debug);
    assert!(!effects.async_pending);
    assert_eq!(collect_values(iter), vec![42]);
}
