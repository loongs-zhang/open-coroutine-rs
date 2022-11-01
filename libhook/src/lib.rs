use open_coroutine::coroutine::Coroutine;
use open_coroutine::scheduler::Scheduler;
use std::os::raw::c_void;
use std::ptr;
use std::time::Duration;

//被hook的系统函数
#[no_mangle]
pub extern "C" fn sleep(secs: libc::c_uint) -> libc::c_uint {
    let rqtp = libc::timespec {
        tv_sec: secs as i64,
        tv_nsec: 0,
    };
    let mut rmtp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    nanosleep(&rqtp, &mut rmtp);
    rmtp.tv_sec as u32
}

#[no_mangle]
pub extern "C" fn usleep(secs: libc::c_uint) -> libc::c_int {
    let secs = secs as i64;
    let sec = secs / 1_000_000;
    let nsec = (secs - sec * 1_000_000) * 1000;
    let rqtp = libc::timespec {
        tv_sec: sec,
        tv_nsec: nsec,
    };
    let mut rmtp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    nanosleep(&rqtp, &mut rmtp)
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn nanosleep(rqtp: *const libc::timespec, rmtp: *mut libc::timespec) -> libc::c_int {
    let nanos_time = unsafe { (*rqtp).tv_sec * 1_000_000_000 + (*rqtp).tv_nsec } as u64;
    let timeout_time = timer::get_timeout_time(Duration::from_nanos(nanos_time));
    Scheduler::current().try_timed_schedule(Duration::from_nanos(nanos_time));
    // 可能schedule完还剩一些时间，此时本地队列没有任务可做
    // 后续考虑work-steal，需要在Scheduler增加timed_schedule实现
    let schedule_finished_time = timer::now();
    let left_time = (timeout_time - schedule_finished_time) as i64;
    if left_time <= 0 {
        unsafe {
            (*rmtp).tv_sec = 0;
            (*rmtp).tv_nsec = 0;
        }
        return 0;
    }
    let sec = left_time / 1_000_000_000;
    let nsec = left_time - sec * 1_000_000_000;
    let rqtp = libc::timespec {
        tv_sec: sec,
        tv_nsec: nsec,
    };
    unsafe {
        //获取原始系统函数nanosleep，后续需要抽成单独的方法
        let original = std::mem::transmute::<
            _,
            extern "C" fn(*const libc::timespec, *mut libc::timespec) -> libc::c_int,
        >(libc::dlsym(libc::RTLD_NEXT, "nanosleep".as_ptr() as _));
        //相当于libc::nanosleep(&rqtp, rmtp)
        original(&rqtp, rmtp)
    }
}

//#[no_mangle]避免rust编译器修改方法名称
#[no_mangle]
pub extern "C" fn coroutine_crate(pointer: &'static mut c_void) {
    let coroutine = unsafe {
        ptr::read_unaligned(
            pointer as *mut _
                as *mut Coroutine<dyn FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>,
        )
    };
    Scheduler::current().submit(coroutine)
}

#[no_mangle]
pub extern "C" fn try_schedule() -> &'static mut c_void {
    let list = Scheduler::current().try_schedule();
    let result = Box::leak(Box::new(list));
    unsafe { &mut *(result as *mut _ as *mut c_void) }
}

#[no_mangle]
pub extern "C" fn schedule() -> &'static mut c_void {
    let list = Scheduler::current().schedule();
    let result = Box::leak(Box::new(list));
    unsafe { &mut *(result as *mut _ as *mut c_void) }
}
