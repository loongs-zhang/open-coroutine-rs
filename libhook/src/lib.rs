use open_coroutine::coroutine::Coroutine;
use open_coroutine::scheduler::Scheduler;
use std::os::raw::c_void;
use std::ptr;
use std::time::Duration;

/**
被hook的系统函数
#[no_mangle]避免rust编译器修改方法名称
epoll like
fcntl由于最后一个参数的类型问题，不支持
todo 待支持io_uring
todo 待完善编译条件
 */
#[no_mangle]
pub fn poll(fds: *mut libc::pollfd, nfds: libc::nfds_t, timeout: libc::c_int) -> libc::c_int {
    todo!()
}

#[no_mangle]
pub fn select(
    nfds: libc::c_int,
    readfs: *mut libc::fd_set,
    writefds: *mut libc::fd_set,
    errorfds: *mut libc::fd_set,
    timeout: *mut libc::timeval,
) -> libc::c_int {
    todo!()
}

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos", target_os = "watchos"))]
#[no_mangle]
pub fn kevent(
    kq: libc::c_int,
    changelist: *const libc::kevent,
    nchanges: libc::c_int,
    eventlist: *mut libc::kevent,
    nevents: libc::c_int,
    timeout: *const libc::timespec,
) -> libc::c_int {
    todo!()
}

#[cfg(linux)]
#[no_mangle]
pub fn epoll_wait(
    epfd: libc::c_int,
    events: *mut libc::epoll_event,
    maxevents: libc::c_int,
    timeout: libc::c_int,
) -> libc::c_int {
    todo!()
}

//socket相关
#[no_mangle]
pub fn setsockopt(
    socket: libc::c_int,
    level: libc::c_int,
    name: libc::c_int,
    value: *const libc::c_void,
    option_len: libc::socklen_t,
) -> libc::c_int {
    todo!()
}

#[no_mangle]
pub fn accept(
    socket: libc::c_int,
    address: *mut libc::sockaddr,
    address_len: *mut libc::socklen_t,
) -> libc::c_int {
    //需要强制把socket设置为非阻塞
    todo!()
}

#[no_mangle]
pub fn connect(
    socket: libc::c_int,
    address: *const libc::sockaddr,
    len: libc::socklen_t,
) -> libc::c_int {
    //需要强制把socket设置为非阻塞
    todo!()
}

#[no_mangle]
pub fn close(fd: libc::c_int) -> libc::c_int {
    todo!()
}

//读数据
#[no_mangle]
pub fn read(fd: libc::c_int, buf: *mut libc::c_void, count: libc::size_t) -> libc::ssize_t {
    todo!()
}

#[no_mangle]
pub fn readv(fd: libc::c_int, iov: *const libc::iovec, iovcnt: libc::c_int) -> libc::ssize_t {
    todo!()
}

#[no_mangle]
pub fn recv(
    socket: libc::c_int,
    buf: *mut libc::c_void,
    len: libc::size_t,
    flags: libc::c_int,
) -> libc::ssize_t {
    todo!()
}

#[no_mangle]
pub fn recvfrom(
    socket: libc::c_int,
    buf: *mut libc::c_void,
    len: libc::size_t,
    flags: libc::c_int,
    addr: *mut libc::sockaddr,
    addrlen: *mut libc::socklen_t,
) -> libc::ssize_t {
    todo!()
}

#[no_mangle]
pub fn recvmsg(fd: libc::c_int, msg: *mut libc::msghdr, flags: libc::c_int) -> libc::ssize_t {
    todo!()
}

//写数据
#[no_mangle]
pub fn write(fd: libc::c_int, buf: *const libc::c_void, count: libc::size_t) -> libc::ssize_t {
    todo!()
}

#[no_mangle]
pub fn writev(fd: libc::c_int, iov: *const libc::iovec, iovcnt: libc::c_int) -> libc::ssize_t {
    todo!()
}

#[no_mangle]
pub fn send(
    socket: libc::c_int,
    buf: *const libc::c_void,
    len: libc::size_t,
    flags: libc::c_int,
) -> libc::ssize_t {
    todo!()
}

#[no_mangle]
pub fn sendto(
    socket: libc::c_int,
    buf: *const libc::c_void,
    len: libc::size_t,
    flags: libc::c_int,
    addr: *const libc::sockaddr,
    addrlen: libc::socklen_t,
) -> libc::ssize_t {
    todo!()
}

#[no_mangle]
pub fn sendmsg(fd: libc::c_int, msg: *const libc::msghdr, flags: libc::c_int) -> libc::ssize_t {
    todo!()
}

//sleep相关
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
