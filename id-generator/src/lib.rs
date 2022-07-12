#[macro_use]
extern crate lazy_static;

use std::sync::atomic::{AtomicUsize, Ordering};

pub struct IdGenerator {}

lazy_static! {
    static ref ID: AtomicUsize = AtomicUsize::new(1);
}

impl IdGenerator {
    pub fn next_id() -> usize {
        ID.fetch_add(1, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use crate::IdGenerator;

    #[test]
    fn test() {
        assert_eq!(1, IdGenerator::next_id());
        assert_eq!(2, IdGenerator::next_id());
        assert_eq!(3, IdGenerator::next_id());
    }
}