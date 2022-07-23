use std::mem;
use std::os::raw::{c_uint, c_void};

//被hook的系统函数
#[no_mangle]
pub extern "C" fn sleep(i: c_uint) -> c_uint {
    println!("hooked sleep {}", i);
    unsafe {
        let pointer = libc::dlsym(libc::RTLD_NEXT as *mut c_void, "sleep".as_ptr() as _);
        //todo 替换为open-coroutine的实现
        //获取原始系统函数
        let original = mem::transmute::<_, extern "C" fn(c_uint) -> c_uint>(pointer);
        //调用原始系统函数
        original(i)
    }
}