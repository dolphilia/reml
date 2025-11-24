use super::{EffectSet, Iter, IterDriver, IterSeed, IterStep, IteratorKind, IteratorStageProfile};

/// 内部ユーティリティ: ドライバと効果タグから `Iter` を構築する。
fn build_iter<T>(
    label: &'static str,
    kind: IteratorKind,
    effects: EffectSet,
    driver: IterDriver<T>,
) -> Iter<T> {
    let stage = IteratorStageProfile::for_kind(kind);
    let seed = IterSeed::new(label, stage.clone(), driver, effects);
    Iter::from_seed(seed)
}

impl<T> Iter<T> {
    /// リストから `Iter` を作成する。
    pub fn from_list(list: impl Into<Vec<T>>) -> Self {
        build_iter(
            "Iter::from_list",
            IteratorKind::CoreIter,
            EffectSet::PURE,
            IterDriver::from_vec(list.into()),
        )
    }

    /// 永続コレクションから Stage = stable の `Iter` を生成する。
    pub fn from_persistent(label: &'static str, values: Vec<T>) -> Self {
        build_iter(
            label,
            IteratorKind::PersistentCollection,
            EffectSet::PURE,
            IterDriver::from_vec(values),
        )
    }

    /// `Result` から `Iter` を作成し、`Ok` のときのみ 1 要素を生成する。
    pub fn from_result<E>(result: std::result::Result<T, E>) -> Self {
        match result {
            Ok(value) => Iter::once(value),
            Err(_) => Iter::empty(),
        }
    }

    /// `FnMut` ベースの生成器から `Iter` を構築する。
    pub fn from_fn<F>(generator: F) -> Self
    where
        F: FnMut() -> Option<T> + Send + 'static,
    {
        let mut generator = generator;
        let driver = IterDriver::stepper(move |_effects| match generator() {
            Some(value) => IterStep::Ready(value),
            None => IterStep::Finished,
        });
        build_iter(
            "Iter::from_fn",
            IteratorKind::CoreIter,
            EffectSet::PURE,
            driver,
        )
    }

    /// 1 要素のみを返す `Iter`。
    pub fn once(value: T) -> Self {
        build_iter(
            "Iter::once",
            IteratorKind::CoreIter,
            EffectSet::PURE,
            IterDriver::from_vec(vec![value]),
        )
    }

    /// `value` を無限に繰り返す `Iter`。
    pub fn repeat(value: T) -> Self
    where
        T: Clone + Send + 'static,
    {
        let driver = IterDriver::stepper(move |_effects| IterStep::Ready(value.clone()));
        build_iter(
            "Iter::repeat",
            IteratorKind::CoreIter,
            EffectSet::PURE,
            driver,
        )
    }

    /// `Iterator::unfold` 相当の `Iter`。
    pub fn unfold<S, F>(state: S, mut f: F) -> Self
    where
        S: Send + 'static,
        F: FnMut(S) -> Option<(T, S)> + Send + 'static,
    {
        let mut slot = Some(state);
        let driver = IterDriver::stepper(move |_effects| match slot.take() {
            Some(state) => match f(state) {
                Some((value, next_state)) => {
                    slot = Some(next_state);
                    IterStep::Ready(value)
                }
                None => IterStep::Finished,
            },
            None => IterStep::Finished,
        });
        build_iter(
            "Iter::unfold",
            IteratorKind::CoreIter,
            EffectSet::PURE,
            driver,
        )
    }

    /// `Result` を返す `unfold`。エラー発生時は `effect {debug}` を記録する。
    pub fn try_unfold<S, E, F>(state: S, mut f: F) -> Self
    where
        S: Send + 'static,
        E: Send + 'static,
        F: FnMut(S) -> Result<Option<(T, S)>, E> + Send + 'static,
    {
        let mut slot = Some(state);
        let driver = IterDriver::stepper(move |_effects| match slot.take() {
            Some(state) => match f(state) {
                Ok(Some((value, next_state))) => {
                    slot = Some(next_state);
                    IterStep::Ready(value)
                }
                Ok(None) => IterStep::Finished,
                Err(_) => IterStep::Finished,
            },
            None => IterStep::Finished,
        });
        build_iter(
            "Iter::try_unfold",
            IteratorKind::CoreIter,
            EffectSet::PURE.with_debug(),
            driver,
        )
    }
}

impl Iter<i64> {
    /// 整数範囲を生成する `Iter<i64>`。
    pub fn range(start: i64, end: i64, step: i64) -> Self {
        let step = if step == 0 { 1 } else { step };
        let increasing = step > 0;
        let mut current = start;
        let driver = IterDriver::stepper(move |_effects| {
            if increasing && current > end {
                return IterStep::Finished;
            }
            if !increasing && current < end {
                return IterStep::Finished;
            }
            let next = current.checked_add(step).unwrap_or(current);
            let value = current;
            current = next;
            IterStep::Ready(value)
        });
        build_iter(
            "Iter::range",
            IteratorKind::CoreIter,
            EffectSet::PURE,
            driver,
        )
    }
}
