use std::collections::{HashMap, VecDeque};
use std::{mem, ptr};
use std::rc::Weak;
use std::sync::RwLock;
use crossbeam_deque::Worker;
use crate::stack::{ProtectedFixedSizeStack, Stack, StackError};

#[derive(Debug)]
pub struct SizedMemoryPool {
    //内存大小
    size: usize,
    //可用的内存池
    available: Worker<ProtectedFixedSizeStack>,
    //正在使用的内存池
    using: VecDeque<ProtectedFixedSizeStack>,
}

unsafe impl Send for SizedMemoryPool {}

unsafe impl Sync for SizedMemoryPool {}

impl SizedMemoryPool {
    pub fn new(size: usize) -> Self {
        SizedMemoryPool {
            size,
            available: Worker::new_fifo(),
            using: VecDeque::new(),
        }
    }

    pub fn allocate(&mut self) -> Result<ProtectedFixedSizeStack, StackError> {
        if self.available.is_empty() {
            //新申请栈
            let stack = ProtectedFixedSizeStack::new(self.size)?;
            self.available.push(stack);
        }
        match self.available.pop() {
            Some(available) => {
                unsafe {
                    self.using.push_back(ptr::read(&available));
                    Ok(ptr::read(&available))
                }
            }
            None => self.allocate()
        }
    }

    fn delete_using(&mut self, stack: &ProtectedFixedSizeStack) {
        //删除using中的元素
        for i in 0..self.using.len() {
            match self.using.get(i) {
                Some(s) => {
                    if s == stack {
                        self.using.remove(i);
                    }
                }
                None => {}
            }
        }
    }

    pub fn revert(&mut self, stack: ProtectedFixedSizeStack) {
        self.delete_using(&stack);
        self.available.push(stack);
    }

    pub fn drop(&mut self, stack: ProtectedFixedSizeStack) {
        self.delete_using(&stack);
        stack.drop();
    }

    pub fn available(&self) -> &Worker<ProtectedFixedSizeStack> {
        &self.available
    }

    pub fn using(&self) -> &VecDeque<ProtectedFixedSizeStack> {
        &self.using
    }
}

lazy_static! {
    static ref MEMORY_POOL: RwLock<HashMap<usize, SizedMemoryPool>> = RwLock::new(HashMap::new());
}

pub fn get_memory_pool(size: usize) -> SizedMemoryPool {
    match MEMORY_POOL.write() {
        Ok(mut map) => {
            match map.get_mut(&size) {
                Some(pool) => {
                    unsafe { ptr::read(pool) }
                }
                None => {
                    map.insert(size, SizedMemoryPool::new(size));
                    unsafe { ptr::read(map.get_mut(&size).unwrap()) }
                }
            }
        }
        Err(_) => get_memory_pool(size)
    }
}

pub fn allocate(size: usize) -> Result<ProtectedFixedSizeStack, StackError> {
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

pub fn revert(stack: ProtectedFixedSizeStack) {
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

pub fn drop(stack: ProtectedFixedSizeStack) {
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

#[cfg(test)]
mod tests {
    use crate::{pools, sys};
    use crate::pools::{MEMORY_POOL, SizedMemoryPool};

    #[test]
    fn test_sized_memory_pool() {
        let mut pool = SizedMemoryPool::new(sys::min_stack_size());
        assert_eq!(0, pool.available().len());
        assert_eq!(0, pool.using().len());
        let stack = pool.allocate().unwrap();
        assert_eq!(0, pool.available().len());
        assert_eq!(1, pool.using().len());
        pool.revert(stack);
        assert_eq!(1, pool.available().len());
        assert_eq!(0, pool.using().len());
    }

    #[test]
    fn test_memory_pool() {
        let size = sys::min_stack_size();
        assert_eq!(0, MEMORY_POOL.read().unwrap().len());
        let stack = pools::allocate(size).unwrap();
        assert_eq!(size, stack.len());
        assert_eq!(1, MEMORY_POOL.read().unwrap().len());
        let pool = pools::get_memory_pool(size);
        assert_eq!(0, pool.available().len());
        assert_eq!(1, pool.using().len());
        pools::revert(stack);
        assert_eq!(1, pool.available().len());
        // fixme
        // assert_eq!(0, pool.using().len());
    }
}