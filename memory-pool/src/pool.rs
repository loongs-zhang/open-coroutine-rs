use std::collections::VecDeque;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::ptr;
use crossbeam_deque::Worker;
use crate::memory::{Memory, MemoryError};
use crate::system;

#[derive(Debug)]
pub struct SizedMemoryPool {
    //内存大小
    size: usize,
    //可用的内存池
    available: Worker<ManuallyDrop<Memory>>,
    //正在使用的内存池
    using: VecDeque<ManuallyDrop<Memory>>,
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

    pub fn allocate(&mut self) -> Result<ManuallyDrop<Memory>, MemoryError> {
        if self.available.is_empty() {
            //新申请栈
            let stack = Memory::new(self.size)?;
            self.available.push(ManuallyDrop::new(stack));
        }
        match self.available.pop() {
            Some(available) => {
                unsafe {
                    self.using.push_back(available);
                    Ok(available)
                }
            }
            None => self.allocate()
        }
    }

    fn delete_using(&mut self, stack: &ManuallyDrop<Memory>) {
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

    pub fn revert(&mut self, stack: ManuallyDrop<Memory>) {
        self.delete_using(&stack);
        self.available.push(stack);
    }

    pub fn drop(&mut self, stack: ManuallyDrop<Memory>) {
        self.delete_using(&stack);
        stack.drop();
    }

    pub fn available(&self) -> &Worker<ManuallyDrop<Memory>> {
        &self.available
    }

    pub fn using(&self) -> &VecDeque<ManuallyDrop<Memory>> {
        &self.using
    }
}

impl Default for SizedMemoryPool {
    fn default() -> Self {
        SizedMemoryPool::new(system::default_size(true))
    }
}


#[cfg(test)]
mod tests {
    use std::ptr;
    use crate::{pool, system};
    use crate::pool::SizedMemoryPool;

    #[test]
    fn test_sized_memory_pool() {
        let size = system::min_size();
        let mut pool = SizedMemoryPool::new(size);
        assert_eq!(0, pool.available().len());
        assert_eq!(0, pool.using().len());

        let stack = pool.allocate().unwrap();
        assert_eq!(size, stack.len());
        assert!(!stack.top().is_null());
        assert!(!stack.bottom().is_null());
        assert!(stack.is_protected());
        unsafe { ptr::write_bytes(stack.bottom() as *mut u8, 0x1d, stack.len()) };

        assert_eq!(0, pool.available().len());
        assert_eq!(1, pool.using().len());
        pool.revert(stack);
        assert_eq!(1, pool.available().len());
        assert_eq!(0, pool.using().len());
    }
}