use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

pub struct IdGenerator {}

static mut ID_MAP: Lazy<RwLock<HashMap<&str, AtomicUsize>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

impl IdGenerator {
    pub fn next_id(key: &'static str) -> usize {
        unsafe {
            match ID_MAP.write() {
                Ok(mut map) => match map.get_mut(key) {
                    Some(id) => id.fetch_add(1, Ordering::SeqCst),
                    None => {
                        map.insert(key, AtomicUsize::new(1));
                        map.get_mut(key).unwrap().fetch_add(1, Ordering::SeqCst)
                    }
                },
                Err(_) => IdGenerator::next_id(key),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::IdGenerator;

    #[test]
    fn test() {
        let key = "global";
        assert_eq!(1, IdGenerator::next_id(key));
        assert_eq!(2, IdGenerator::next_id(key));
        assert_eq!(3, IdGenerator::next_id(key));
    }
}
