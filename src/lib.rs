use std::any::Any;

pub trait Frame {}

pub trait Stack {
    /// 出栈
    fn pop(&self) -> dyn Frame;

    /// 入栈
    fn push(&self, frame: dyn Frame);

    /// 获取当前栈顶指针
    fn top(&self) -> &dyn Frame;

    /// 获取当前栈底部指针
    fn bottom(&self) -> &dyn Frame;

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

pub trait MainCoroutine {
    /// 创建一个主协程
    fn create() -> Self;

    /// 将执行权交给另一个非主协程，指定时间后执行权将回到主协程，抢占调度
    fn resume(coroutine: &dyn Coroutine, timeout: usize);

    /// 销毁协程
    fn destroy(coroutine: &dyn Coroutine);

    /// 主协程执行完毕
    fn exit(&self);
}

pub trait Coroutine {
    /// 创建一个协程
    fn create(main: Option<&dyn MainCoroutine>,
              stack: &dyn Stack,
              function: dyn FnOnce<dyn Any>,
              param_pointer: usize) -> Self;

    /// 非主协程将执行权交还给主协程
    fn yields(&self);

    /// 非主协程执行完毕
    fn exit(&self);

    /// 获取协程的当前状态
    fn state(&self);

    /// 设置协程参数
    fn set_param(&self, param_pointer: usize);

    /// 获取协程参数
    fn get_param(&self) -> usize;
}

/// hook系统调用，此功能仅对付费用户开放，注意加密
/// ```
/// //这样可以拿到系统函数
/// let read = unsafe { libc::dlsym(libc::RTLD_NEXT, "read".as_ptr() as *const _) };
/// ```
pub trait SystemCallHooker {
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