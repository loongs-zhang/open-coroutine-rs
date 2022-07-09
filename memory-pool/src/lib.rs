#[macro_use]
extern crate lazy_static;

pub mod pool;

pub mod memory;

mod system;

use std::mem::ManuallyDrop;
use std::sync::RwLock;
use std::collections::HashMap;
use std::ptr::NonNull;
use crate::memory::{Memory, MemoryError};
use crate::pool::SizedMemoryPool;

lazy_static! {
    static ref MEMORY_POOL: RwLock<HashMap<usize, SizedMemoryPool>> = RwLock::new(HashMap::new());
}

pub fn get_memory_pool(size: usize) -> Option<NonNull<SizedMemoryPool>> {
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

pub fn allocate(size: usize) -> Result<ManuallyDrop<Memory>, MemoryError> {
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

pub fn revert(stack: ManuallyDrop<Memory>) {
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

pub fn drop(stack: ManuallyDrop<Memory>) {
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

pub fn default() -> Result<ManuallyDrop<Memory>, MemoryError> {
    allocate(system::default_size(true))
}

#[cfg(test)]
mod tests {
    use crate::{allocate, get_memory_pool, MEMORY_POOL, revert};
    use crate::{pool, system};
    use crate::pool::SizedMemoryPool;

    #[test]
    fn test_memory_pool() {
        let size = system::min_size();
        assert_eq!(0, MEMORY_POOL.read().unwrap().len());
        let stack = allocate(size).unwrap();
        assert_eq!(size, stack.len());
        assert_eq!(1, MEMORY_POOL.read().unwrap().len());
        let pool = get_memory_pool(size).unwrap();
        unsafe {
            assert_eq!(0, pool.as_ref().available().len());
            assert_eq!(1, pool.as_ref().using().len());
            revert(stack);
            assert_eq!(1, pool.as_ref().available().len());
            assert_eq!(0, pool.as_ref().using().len());
        }
    }
}