//! Global string interner for field names.
//!
//! Field names like "level", "msg", "ts" appear in every record.
//! Without interning, a 1M-record file allocates 1M separate "level" strings.
//! With interning, all records share the same `Arc<str>` allocation per unique name,
//! reducing heap usage by O(records × avg_fields × avg_field_name_len).
//!
//! The global pool uses a `RwLock<HashMap>` so that:
//! - Concurrent reads (hot path after warm-up) take a shared lock — no contention.
//! - Concurrent writes (first time a field name is seen) take an exclusive lock.
//!
//! In practice, after the first record of a NDJSON file, all field names are cached
//! and subsequent records only use the read path.

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock},
};

type Pool = RwLock<HashMap<Box<str>, Arc<str>>>;

static POOL: OnceLock<Pool> = OnceLock::new();

fn pool() -> &'static Pool {
    POOL.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Intern `s`, returning a shared `Arc<str>` deduplicated across all callers.
///
/// All calls with the same string value return an `Arc` that points to the same
/// heap allocation, saving memory for repeated field names across many records.
pub fn intern(s: &str) -> Arc<str> {
    // Fast path: already in pool — only a shared read lock needed.
    {
        let guard = pool().read().expect("intern pool read lock poisoned");
        if let Some(arc) = guard.get(s) {
            return Arc::clone(arc);
        }
    }
    // Slow path: insert — requires exclusive write lock.
    let mut guard = pool().write().expect("intern pool write lock poisoned");
    // Double-check: another thread may have inserted between unlock and re-lock.
    if let Some(arc) = guard.get(s) {
        return Arc::clone(arc);
    }
    let key: Box<str> = s.into();
    let val: Arc<str> = Arc::from(s);
    guard.insert(key, Arc::clone(&val));
    val
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn same_string_returns_same_arc() {
        let a = intern("level");
        let b = intern("level");
        // Both Arcs should point to the same underlying allocation.
        assert!(
            ptr::eq(a.as_ptr(), b.as_ptr()),
            "expected same Arc allocation"
        );
    }

    #[test]
    fn different_strings_return_different_arcs() {
        let a = intern("level");
        let b = intern("service");
        assert!(!ptr::eq(a.as_ptr(), b.as_ptr()));
    }

    #[test]
    fn interned_value_equals_original() {
        let arc = intern("msg");
        assert_eq!(&*arc, "msg");
    }
}
