//! Locks of the service. A request that panics ends alone (the service
//! unwinds), and the requests after it take the state that it left.

use std::sync::{Mutex, MutexGuard, TryLockError};

/// Waits for `mutex`.
pub fn lock<T: ?Sized>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        mutex.clear_poison();
        poisoned.into_inner()
    })
}

/// Takes `mutex` when no other request holds it.
pub fn try_lock<T: ?Sized>(mutex: &Mutex<T>) -> Option<MutexGuard<'_, T>> {
    match mutex.try_lock() {
        Ok(guard) => Some(guard),
        Err(TryLockError::WouldBlock) => None,
        Err(TryLockError::Poisoned(poisoned)) => {
            mutex.clear_poison();
            Some(poisoned.into_inner())
        }
    }
}
