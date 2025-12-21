use once_cell::sync::Lazy;
use std::sync::{Mutex, MutexGuard};

static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub fn lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap()
}
