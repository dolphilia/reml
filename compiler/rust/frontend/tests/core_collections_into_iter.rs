use reml_runtime_ffi::core_prelude::{
    collectors::{List, Map, Set},
    iter::Iter,
};

#[test]
fn list_into_iter_round_trip() {
    let list = List::from_vec(vec![1, 2, 3]);
    let collected: Vec<_> = list.into_iter().collect();
    assert_eq!(collected, vec![1, 2, 3]);
}

#[test]
fn map_into_iter_round_trip() {
    let map = Map::new()
        .insert("beta".to_string(), 2)
        .insert("alpha".to_string(), 1);
    let entries: Vec<_> = map.into_iter().collect();
    let expected = vec![("alpha".to_string(), 1), ("beta".to_string(), 2)];
    assert_eq!(entries, expected);
}

#[test]
fn set_into_iter_round_trip() {
    let set = Set::new().insert(3).insert(1).insert(2);
    let values: Vec<_> = set.into_iter().collect();
    assert_eq!(values, vec![1, 2, 3]);
}

#[test]
fn persistent_iter_stage_snapshot() {
    let values = vec![10, 20, 30];
    let iter = Iter::from_persistent("List::into_iter", values.clone());
    let stage = iter.stage_snapshot("core.collections.into_iter");
    assert_eq!(stage.actual, "stable");
    assert_eq!(stage.required.stage, "stable");
    assert_eq!(stage.required.mode, "exact");
    assert_eq!(stage.kind, "persistent_collection");
    assert_eq!(stage.capability, Some("core.iter.persistent_collection"));
    let collected: Vec<_> = iter.into_iter().collect();
    assert_eq!(collected, values);
}
