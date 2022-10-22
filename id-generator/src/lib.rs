use std::sync::atomic::{AtomicUsize, Ordering};
use once_cell::sync::Lazy;

pub struct IdGenerator {}

static mut ID: Lazy<AtomicUsize> = Lazy::new(|| {
    AtomicUsize::new(1)
});

impl IdGenerator {
    pub fn next_id() -> usize {
        unsafe { ID.fetch_add(1, Ordering::SeqCst) }
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