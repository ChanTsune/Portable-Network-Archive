//! LRU cache for derived password keys.

use password_hash::Output;
use std::fmt;

/// LRU cache of derived keys. Capacity 8, keyed by PHSF string.
///
/// # Safety assumption
///
/// This cache assumes all lookups use the same password.
/// The cache key is the PHSF string only (not the password).
/// Do not reuse a `KeyCache` across different passwords.
#[derive(Clone, Default)]
pub(crate) struct KeyCache {
    entries: Vec<(String, Output)>,
}

impl KeyCache {
    const CAPACITY: usize = 8;

    /// Look up a cached derived key by PHSF string.
    /// Moves the hit entry to front (MRU) on success.
    pub(crate) fn get(&mut self, phsf: &str) -> Option<Output> {
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == phsf) {
            let entry = self.entries.remove(pos);
            let output = entry.1;
            self.entries.insert(0, entry);
            Some(output)
        } else {
            None
        }
    }

    /// Insert a derived key. Evicts LRU (last) entry if at capacity.
    pub(crate) fn insert(&mut self, phsf: String, key: Output) {
        self.entries.retain(|(k, _)| k != &phsf);
        if self.entries.len() >= Self::CAPACITY {
            self.entries.pop();
        }
        self.entries.insert(0, (phsf, key));
    }
}

impl fmt::Debug for KeyCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyCache")
            .field("len", &self.entries.len())
            .field("capacity", &Self::CAPACITY)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_output(seed: u8) -> Output {
        Output::new(&[seed; 32]).unwrap()
    }

    #[test]
    fn get_returns_none_for_empty_cache() {
        let mut cache = KeyCache::default();
        assert!(cache.get("$argon2id$...").is_none());
    }

    #[test]
    fn insert_and_get_returns_cached_key() {
        let mut cache = KeyCache::default();
        let key = dummy_output(1);
        cache.insert("phsf1".to_string(), key);
        assert_eq!(cache.get("phsf1").unwrap(), key);
    }

    #[test]
    fn get_miss_for_different_phsf() {
        let mut cache = KeyCache::default();
        cache.insert("phsf1".to_string(), dummy_output(1));
        assert!(cache.get("phsf2").is_none());
    }

    #[test]
    fn evicts_lru_at_capacity() {
        let mut cache = KeyCache::default();
        for i in 0..8 {
            cache.insert(format!("phsf{i}"), dummy_output(i));
        }
        // Insert 9th — should evict the LRU (phsf0 at back)
        cache.insert("phsf_new".to_string(), dummy_output(99));
        assert!(cache.get("phsf_new").is_some());
        // phsf0 was inserted first and is now at back → evicted
        assert!(cache.get("phsf0").is_none());
    }

    #[test]
    fn get_promotes_entry_to_mru() {
        let mut cache = KeyCache::default();
        for i in 0..8 {
            cache.insert(format!("phsf{i}"), dummy_output(i));
        }
        // Access phsf0 to promote it
        assert!(cache.get("phsf0").is_some());
        // Insert new → should evict phsf1 (now LRU), not phsf0
        cache.insert("phsf_new".to_string(), dummy_output(99));
        assert!(cache.get("phsf0").is_some());
        assert!(cache.get("phsf1").is_none());
    }

    #[test]
    fn insert_same_key_updates_value() {
        let mut cache = KeyCache::default();
        let key1 = dummy_output(1);
        let key2 = dummy_output(2);
        cache.insert("phsf1".to_string(), key1);
        cache.insert("phsf1".to_string(), key2);
        assert_eq!(cache.get("phsf1").unwrap(), key2);
    }

    #[test]
    fn debug_redacts_keys() {
        let mut cache = KeyCache::default();
        cache.insert("phsf1".to_string(), dummy_output(1));
        let debug = format!("{cache:?}");
        assert!(debug.contains("len: 1"));
        assert!(debug.contains("capacity: 8"));
        assert!(!debug.contains("phsf1"));
    }
}
