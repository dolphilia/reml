use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

/// Thread-Local に再利用される IO バッファ。
///
/// Core.IO のヘルパ（`copy` や `BufferedReader`）で使う 64KiB バッファを
/// 何度も確保しないようにするための薄い RAII ラッパ。`Drop` でプールへ戻す。
#[derive(Debug)]
pub(crate) struct IoCopyBuffer {
    buffer: Option<Vec<u8>>,
}

thread_local! {
    static BUFFER_POOL: RefCell<Vec<Vec<u8>>> = RefCell::new(Vec::new());
}

impl IoCopyBuffer {
    /// 最低 `min_capacity` バイトのバッファを借りる。
    pub fn lease(min_capacity: usize) -> Self {
        let mut buffer = BUFFER_POOL.with(|pool| {
            let mut pool = pool.borrow_mut();
            if let Some((index, _)) = pool
                .iter()
                .enumerate()
                .find(|(_, existing)| existing.len() >= min_capacity)
            {
                pool.swap_remove(index)
            } else {
                Vec::with_capacity(min_capacity)
            }
        });
        if buffer.len() < min_capacity {
            buffer.resize(min_capacity, 0);
        }
        Self {
            buffer: Some(buffer),
        }
    }

    /// 現在のバッファ長（= 利用可能容量）を返す。
    pub fn len(&self) -> usize {
        self.buffer.as_ref().map(|buf| buf.len()).unwrap_or(0)
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Deref for IoCopyBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.buffer
            .as_ref()
            .map(|buffer| buffer.as_slice())
            .expect("IoCopyBuffer is always initialized")
    }
}

impl DerefMut for IoCopyBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
            .as_mut()
            .map(|buffer| buffer.as_mut_slice())
            .expect("IoCopyBuffer is always initialized")
    }
}

impl Drop for IoCopyBuffer {
    fn drop(&mut self) {
        if let Some(mut buffer) = self.buffer.take() {
            BUFFER_POOL.with(|pool| {
                let mut pool = pool.borrow_mut();
                if pool.len() < 8 {
                    buffer.fill(0);
                    pool.push(buffer);
                }
            });
        }
    }
}
