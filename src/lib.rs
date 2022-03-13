use std::any::Any;
use std::cell::Ref;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::Rc;

pub mod api;
mod register;
pub mod coroutine;

pub trait Stack {
    /// 获取当前栈顶指针
    fn top(&self) -> *mut c_void;

    /// 获取当前栈底部指针
    fn bottom(&self) -> *mut c_void;

    /// 栈的总大小
    fn size(&self) -> usize;

    /// 已使用栈的大小
    fn used(&self) -> usize;

    /// 剩余栈大小
    fn remain(&self) -> usize;

    /// 扩容为原来的2倍
    fn resize(&self);

    /// 缩容为原来的1/2
    fn reduce(&self);

    /// 清理栈
    fn clear(&self);
}

/// hook系统调用，此功能仅对付费用户开放，注意加密
/// ```
/// //这样可以拿到系统函数
/// let read = unsafe { libc::dlsym(libc::RTLD_NEXT, "read".as_ptr() as *const _) };
/// ```
pub trait SystemCallHooks {
    fn hook_system_call(name: &str);

    fn hook_sleep();

    fn hook_socket();

    fn hook_connect();

    fn hook_close();

    fn hook_read();

    fn hook_write();

    fn hook_sendto();

    fn hook_recvfrom();

    fn hook_send();

    fn hook_recv();

    fn hook_poll();

    fn hook_setsockopt();

    fn hook_fcntl();

    fn hook_setenv();

    fn hook_unsetenv();

    fn hook_getenv();

    /// hook __res_state
    fn hook_res_state();

    fn hook_gethostbyname();
}

/// 调度器
pub trait Scheduler {
    /// 一次调度
    fn schedule();
}