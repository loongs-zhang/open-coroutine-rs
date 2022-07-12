use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_void;
use std::ptr;

#[derive(Debug)]
pub struct ObjectList {
    inner: VecDeque<*mut c_void>,
}

fn convert<T>(pointer: *mut c_void) -> Option<T> {
    unsafe {
        let node = Box::from_raw(pointer as *mut T);
        Some(ptr::read(node.deref()))
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
                    let result = ptr::read(value) as *mut T;
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
                    let mut result = ptr::read(value) as *mut T;
                    Some(&mut *result)
                }
            }
            None => None
        }
    }

    pub fn push_front<T>(&mut self, element: T) {
        let ptr = Box::leak(Box::new(element));
        self.inner.push_front(ptr as *mut _ as *mut c_void);
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
        match self.inner.pop_front() {
            Some(pointer) => {
                Some(pointer)
            }
            None => None
        }
    }

    pub fn back<T>(&mut self) -> Option<&T> {
        match self.inner.back() {
            Some(value) => {
                unsafe {
                    let result = ptr::read(value) as *mut T;
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
                    let mut result = ptr::read(value) as *mut T;
                    Some(&mut *result)
                }
            }
            None => None
        }
    }

    pub fn push_back<T>(&mut self, element: T) {
        let ptr = Box::leak(Box::new(element));
        self.inner.push_back(ptr as *mut _ as *mut c_void);
    }

    pub fn pop_back<T>(&mut self) -> Option<T> {
        match self.inner.pop_back() {
            Some(pointer) => {
                convert(pointer)
            }
            None => None
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn get<T>(&self, index: usize) -> Option<&T> {
        match self.inner.get(index) {
            Some(val) => {
                unsafe {
                    let result = ptr::read(val) as *mut T;
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
                    let result = ptr::read(val) as *mut T;
                    Some(&mut *result)
                }
            }
            None => None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::ObjectList;

    #[test]
    fn test() {
        let mut list = ObjectList::new();
        assert!(list.is_empty());
        list.push_back(1);
        assert_eq!(&1, list.front().unwrap());
        assert!(!list.is_empty());
        list.push_back(true);
        assert_eq!(&true, list.back().unwrap());

        assert_eq!(&1, list.get(0).unwrap());
        assert_eq!(&true, list.get_mut(1).unwrap());

        let b: bool = list.pop_back().unwrap();
        assert_eq!(true, b);
        let n: i32 = list.pop_back().unwrap();
        assert_eq!(1, n);
    }
}