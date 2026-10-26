//! Open documents (or editor sessions) by id (design 002 E9) and running loads that can be
//! cancelled.

use crate::error::{FsError, FsErrorCode};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

pub type Handle<T> = Arc<Mutex<T>>;

#[derive(Debug)]
pub struct Registry<T> {
    next: AtomicU64,
    docs: Mutex<HashMap<u64, Handle<T>>>,
    loads: Mutex<HashMap<u64, Arc<AtomicBool>>>,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self {
            next: AtomicU64::new(0),
            docs: Mutex::new(HashMap::new()),
            loads: Mutex::new(HashMap::new()),
        }
    }
}

/// A poisoned lock only means another thread panicked – the data is still usable.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl<T> Registry<T> {
    /// The id the next [`Registry::insert`] will use.
    pub fn reserve_id(&self) -> u64 {
        self.next.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn insert(&self, id: u64, value: T) {
        lock(&self.docs).insert(id, Arc::new(Mutex::new(value)));
    }

    pub fn get(&self, id: u64) -> Result<Handle<T>, FsError> {
        lock(&self.docs).get(&id).cloned().ok_or_else(|| {
            FsError::new(FsErrorCode::UnknownDocument, format!("document {id}"), None)
        })
    }

    pub fn remove(&self, id: u64) -> bool {
        lock(&self.docs).remove(&id).is_some()
    }

    pub fn len(&self) -> usize {
        lock(&self.docs).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Registers a load; the returned flag is set by [`Registry::cancel_load`].
    pub fn start_load(&self, load_id: u64) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        lock(&self.loads).insert(load_id, flag.clone());
        flag
    }

    pub fn finish_load(&self, load_id: u64) {
        lock(&self.loads).remove(&load_id);
    }

    pub fn cancel_load(&self, load_id: u64) {
        if let Some(flag) = lock(&self.loads).get(&load_id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

/// Locks an entry (see [`lock`]).
pub fn lock_doc<T>(doc: &Handle<T>) -> MutexGuard<'_, T> {
    lock(doc)
}
