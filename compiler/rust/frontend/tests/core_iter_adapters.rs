use reml_runtime_ffi::core_prelude::iter::{BufferStrategy, Iter};

fn collect_values<T>(iter: Iter<T>) -> Vec<T> {
    iter.collect_vec()
        .expect("VecCollector should not fail")
        .into_parts()
        .0
}

#[test]
fn iter_map_filter_pipeline() {
    let iter = Iter::from_list(vec![1, 2, 3, 4])
        .map(|value| value * 2)
        .filter(|value| *value % 4 == 0);
    assert_eq!(collect_values(iter), vec![4, 8]);
}

#[test]
fn iter_filter_map_skips_invalid() {
    let iter = Iter::from_list(vec!["1", "x", "3"]).filter_map(|value| value.parse::<i32>().ok());
    assert_eq!(collect_values(iter), vec![1, 3]);
}

#[test]
fn iter_flat_map_expands_sequences() {
    let iter = Iter::from_list(vec![1, 2, 3]).flat_map(|value| {
        let repeated = vec![value, value * 10];
        Iter::from_list(repeated)
    });
    assert_eq!(collect_values(iter), vec![1, 10, 2, 20, 3, 30]);
}

#[test]
fn iter_scan_tracks_running_totals() {
    let iter = Iter::from_list(vec![1, 2, 3, 4]).scan(0, |state, value| {
        *state += value;
        Some(*state)
    });
    assert_eq!(collect_values(iter), vec![1, 3, 6, 10]);
}

#[test]
fn iter_take_drop_enumerate() {
    let iter = Iter::from_list(vec![10, 20, 30, 40])
        .drop(1)
        .take(2)
        .enumerate();
    assert_eq!(collect_values(iter), vec![(0, 20), (1, 30)]);
}

#[test]
fn iter_zip_pairs_sequences() {
    let left = Iter::from_list(vec![1, 2, 3]);
    let right = Iter::from_list(vec!["a", "b", "c"]);
    let zipped = left.zip(right);
    assert_eq!(collect_values(zipped), vec![(1, "a"), (2, "b"), (3, "c")]);
}

#[test]
fn iter_buffered_sets_mem_effects() {
    let iter = Iter::from_list(vec![1, 2, 3]).buffered(2, BufferStrategy::DropOldest);
    assert_eq!(collect_values(iter.clone()), vec![1, 2, 3]);
    let labels = iter.effect_labels();
    assert!(labels.mem, "buffered iter should flag mem effect");
    assert_eq!(labels.mem_bytes, 2);
}
