use std::os::raw::{c_uint, c_void};
use std::ptr;
use std::time::Duration;
use object_list::ObjectList;
use open_coroutine::coroutine::Coroutine;
use open_coroutine::scheduler::Scheduler;

//被hook的系统函数
#[no_mangle]
pub extern "C" fn sleep(i: c_uint) -> c_uint {
    println!("hooked sleep {}", i);
    Scheduler::current().timed_schedule(Duration::from_secs(i as u64));
    0
}

//#[no_mangle]避免rust编译器修改方法名称
#[no_mangle]
pub extern "C" fn coroutine_crate(pointer: *mut c_void) {
    let coroutine = unsafe {
        ptr::read_unaligned(pointer as
            *mut Coroutine<dyn FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>)
    };
    Scheduler::current().push(coroutine)
}

#[no_mangle]
pub extern "C" fn try_schedule() -> ObjectList {
    Scheduler::current().try_schedule()
}

#[no_mangle]
pub extern "C" fn schedule() -> ObjectList {
    Scheduler::current().schedule()
}