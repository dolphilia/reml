//! Core.Async で参照される opaque 型の土台。

use std::marker::PhantomData;

/// Reml の `Future<T>` を表す opaque 型。
#[derive(Debug)]
pub struct Future<T> {
    _marker: PhantomData<T>,
}

/// Reml の `AsyncStream<T>` を表す opaque 型。
#[derive(Debug)]
pub struct AsyncStream<T> {
    _marker: PhantomData<T>,
}
