use std::collections::VecDeque;
use std::os::raw::c_void;
use std::ptr;
use crossbeam_deque::{Steal, Worker};

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ObjectList {
    inner: VecDeque<*mut c_void>,
}

fn convert<T>(pointer: *mut c_void) -> Option<T> {
    unsafe {
        let node = Box::from_raw(pointer as *mut T);
        Some(*node)
    }
}

impl ObjectList {
    pub fn new() -> Self {
        ObjectList { inner: VecDeque::new() }
    }

    pub fn front<T>(&mut self) -> Option<&T> {
        match self.inner.front() {
            Some(value) => {
                unsafe {
                    let result = ptr::read_unaligned(value) as *mut T;
                    Some(&*result)
                }
            }
            None => None
        }
    }

    pub fn front_mut<T>(&mut self) -> Option<&mut T> {
        match self.inner.front_mut() {
            Some(value) => {
                unsafe {
                    let result = ptr::read_unaligned(value) as *mut T;
                    Some(&mut *result)
                }
            }
            None => None
        }
    }

    pub fn front_mut_raw(&mut self) -> Option<*mut c_void> {
        match self.inner.front_mut() {
            Some(value) => {
                unsafe { Some(ptr::read_unaligned(value)) }
            }
            None => None
        }
    }

    pub fn push_front<T>(&mut self, element: T) {
        let ptr = Box::leak(Box::new(element));
        self.inner.push_front(ptr as *mut _ as *mut c_void);
    }

    pub fn push_front_raw(&mut self, ptr: *mut c_void) {
        self.inner.push_front(ptr);
    }

    pub fn pop_front<T>(&mut self) -> Option<T> {
        match self.inner.pop_front() {
            Some(pointer) => {
                convert(pointer)
            }
            None => None
        }
    }

    /// 如果是闭包，还是要获取裸指针再手动转换，不然类型有问题
    pub fn pop_front_raw(&mut self) -> Option<*mut c_void> {
        self.inner.pop_front()
    }

    pub fn back<T>(&mut self) -> Option<&T> {
        match self.inner.back() {
            Some(value) => {
                unsafe {
                    let result = ptr::read_unaligned(value) as *mut T;
                    Some(&*result)
                }
            }
            None => None
        }
    }

    pub fn back_mut<T>(&mut self) -> Option<&mut T> {
        match self.inner.back_mut() {
            Some(value) => {
                unsafe {
                    let result = ptr::read_unaligned(value) as *mut T;
                    Some(&mut *result)
                }
            }
            None => None
        }
    }

    pub fn back_mut_raw(&mut self) -> Option<*mut c_void> {
        match self.inner.back_mut() {
            Some(value) => {
                unsafe { Some(ptr::read_unaligned(value)) }
            }
            None => None
        }
    }

    pub fn push_back<T>(&mut self, element: T) {
        let ptr = Box::leak(Box::new(element));
        self.inner.push_back(ptr as *mut _ as *mut c_void);
    }

    pub fn push_back_raw(&mut self, ptr: *mut c_void) {
        self.inner.push_back(ptr);
    }

    pub fn pop_back<T>(&mut self) -> Option<T> {
        match self.inner.pop_back() {
            Some(pointer) => {
                convert(pointer)
            }
            None => None
        }
    }

    /// 如果是闭包，还是要获取裸指针再手动转换，不然类型有问题
    pub fn pop_back_raw(&mut self) -> Option<*mut c_void> {
        self.inner.pop_back()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn get<T>(&self, index: usize) -> Option<&T> {
        match self.inner.get(index) {
            Some(val) => {
                unsafe {
                    let result = ptr::read_unaligned(val) as *mut T;
                    Some(&*result)
                }
            }
            None => None
        }
    }

    pub fn get_mut<T>(&mut self, index: usize) -> Option<&mut T> {
        match self.inner.get_mut(index) {
            Some(val) => {
                unsafe {
                    let result = ptr::read_unaligned(val) as *mut T;
                    Some(&mut *result)
                }
            }
            None => None
        }
    }

    pub fn get_mut_raw(&mut self, index: usize) -> Option<*mut c_void> {
        match self.inner.get_mut(index) {
            Some(pointer) => {
                unsafe { Some(ptr::read_unaligned(pointer)) }
            }
            None => None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn move_front_to_back(&mut self) {
        match self.inner.pop_front() {
            Some(pointer) => {
                self.inner.push_back(pointer)
            }
            None => {}
        }
    }
}

impl AsRef<ObjectList> for ObjectList {
    fn as_ref(&self) -> &ObjectList {
        &*self
    }
}

impl AsMut<ObjectList> for ObjectList {
    fn as_mut(&mut self) -> &mut ObjectList {
        &mut *self
    }
}

#[derive(Debug)]
pub struct StealableObjectList {
    //todo add head/tail field
    inner: Worker<*mut c_void>,
}

impl StealableObjectList {
    pub fn new() -> Self {
        StealableObjectList { inner: Worker::new_fifo() }
    }

    pub fn push_back<T>(&mut self, element: T) {
        let ptr = Box::leak(Box::new(element));
        self.inner.push(ptr as *mut _ as *mut c_void);
    }

    pub fn pop_front<T>(&mut self) -> Option<T> {
        match self.inner.pop() {
            Some(pointer) => {
                convert(pointer)
            }
            None => None
        }
    }

    /// 如果是闭包，还是要获取裸指针再手动转换，不然类型有问题
    pub fn pop_front_raw(&mut self) -> Option<*mut c_void> {
        self.inner.pop()
    }

    pub fn move_front_to_back(&mut self) {
        match self.inner.pop() {
            Some(pointer) => {
                self.inner.push(pointer)
            }
            None => {}
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn steal_to(&self, dest: &StealableObjectList) -> Steal<()> {
        self.inner.stealer().steal_batch(&dest.inner)
    }
}

impl AsRef<StealableObjectList> for StealableObjectList {
    fn as_ref(&self) -> &StealableObjectList {
        &*self
    }
}

impl AsMut<StealableObjectList> for StealableObjectList {
    fn as_mut(&mut self) -> &mut StealableObjectList {
        &mut *self
    }
}

#[cfg(test)]
mod tests {
    use crate::{ObjectList, StealableObjectList};

    #[test]
    fn test() {
        let mut list = ObjectList::new();
        assert!(list.is_empty());
        list.push_back(1);
        assert_eq!(&1, list.front().unwrap());
        assert_eq!(&1, list.front().unwrap());
        assert!(!list.is_empty());
        list.push_back(true);
        assert_eq!(&true, list.back().unwrap());
        assert_eq!(&true, list.back().unwrap());

        assert_eq!(&1, list.get(0).unwrap());
        assert_eq!(&1, list.get(0).unwrap());
        assert_eq!(&true, list.get_mut(1).unwrap());
        assert_eq!(&true, list.get_mut(1).unwrap());

        let b: bool = list.pop_back().unwrap();
        assert_eq!(true, b);
        let n: i32 = list.pop_back().unwrap();
        assert_eq!(1, n);
    }

    #[test]
    fn test_stealable() {
        let mut list = StealableObjectList::new();
        assert!(list.is_empty());
        list.push_back(1);
        assert!(!list.is_empty());
        list.push_back(true);
        assert_eq!(2, list.len());

        let n: i32 = list.pop_front().unwrap();
        assert_eq!(1, n);
        let b: bool = list.pop_front().unwrap();
        assert_eq!(true, b);
    }
}