pub mod pool;

pub mod memory;

mod system;

use std::mem::ManuallyDrop;
use std::sync::RwLock;
use std::collections::HashMap;
use std::ptr::NonNull;
use once_cell::sync::Lazy;
use crate::memory::{Memory, MemoryError};
use crate::pool::SizedMemoryPool;

static mut MEMORY_POOL: Lazy<RwLock<HashMap<usize, SizedMemoryPool>>> = Lazy::new(|| {
    RwLock::new(HashMap::new())
});

pub fn get_memory_pool(size: usize) -> Option<NonNull<SizedMemoryPool>> {
    unsafe {
        match MEMORY_POOL.write() {
            Ok(mut map) => {
                match map.get_mut(&size) {
                    Some(pool) => {
                        NonNull::new(pool as *mut _ as *mut SizedMemoryPool)
                    }
                    None => None
                }
            }
            Err(_) => None
        }
    }
}

pub fn allocate(size: usize) -> Result<ManuallyDrop<Memory>, MemoryError> {
    unsafe {
        match MEMORY_POOL.write() {
            Ok(mut map) => {
                match map.get_mut(&size) {
                    Some(pool) => {
                        pool.allocate()
                    }
                    None => {
                        map.insert(size, SizedMemoryPool::new(size));
                        map.get_mut(&size).unwrap().allocate()
                    }
                }
            }
            Err(_) => allocate(size)
        }
    }
}

pub fn revert(stack: ManuallyDrop<Memory>) {
    unsafe {
        match MEMORY_POOL.write() {
            Ok(mut map) => {
                match map.get_mut(&stack.len()) {
                    Some(pool) => {
                        pool.revert(stack);
                    }
                    None => {}
                }
            }
            Err(_) => revert(stack)
        }
    }
}

pub fn drop(stack: ManuallyDrop<Memory>) {
    unsafe {
        match MEMORY_POOL.write() {
            Ok(mut map) => {
                match map.get_mut(&stack.len()) {
                    Some(pool) => {
                        pool.drop(stack);
                    }
                    None => {}
                }
            }
            Err(_) => drop(stack)
        }
    }
}

pub fn default() -> Result<ManuallyDrop<Memory>, MemoryError> {
    allocate(system::default_size(true))
}

#[cfg(test)]
mod tests {
    use crate::{allocate, get_memory_pool, MEMORY_POOL, revert};
    use crate::system;

    #[test]
    fn test_memory_pool() {
        unsafe {
            let size = system::min_size();
            assert_eq!(0, MEMORY_POOL.read().unwrap().len());
            let stack = allocate(size).unwrap();
            assert_eq!(size, stack.len());
            assert_eq!(1, MEMORY_POOL.read().unwrap().len());
            let pool = get_memory_pool(size).unwrap();
            assert_eq!(0, pool.as_ref().available().len());
            assert_eq!(1, pool.as_ref().using().len());
            revert(stack);
            assert_eq!(1, pool.as_ref().available().len());
            assert_eq!(0, pool.as_ref().using().len());
        }
    }
}