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
    let timeout_time = timer::get_timeout_time(Duration::from_secs(i as u64));
    Scheduler::current().try_timed_schedule(Duration::from_secs(i as u64));
    let schedule_finished_time = timer::now();
    // 可能schedule完还剩一些时间，此时本地队列没有任务可做
    // 后续考虑work-steal，需要在Scheduler增加timed_schedule实现
    let left_time = (timeout_time - schedule_finished_time) as i64;
    if left_time <= 0 {
        return 0;
    }
    let rqtp = libc::timespec { tv_sec: 0, tv_nsec: left_time };
    let mut rmtp = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::nanosleep(&rqtp, &mut rmtp) };
    rmtp.tv_sec as u32
}

//#[no_mangle]避免rust编译器修改方法名称
#[no_mangle]
pub extern "C" fn coroutine_crate(pointer: *mut c_void) {
    let coroutine = unsafe {
        ptr::read_unaligned(pointer as
            *mut Coroutine<dyn FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>)
    };
    Scheduler::current().submit(coroutine)
}

#[no_mangle]
pub extern "C" fn try_schedule() -> ObjectList {
    Scheduler::current().try_schedule()
}

#[no_mangle]
pub extern "C" fn schedule() -> ObjectList {
    Scheduler::current().schedule()
}