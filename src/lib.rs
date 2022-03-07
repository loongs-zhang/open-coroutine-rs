use std::any::Any;

pub trait Frame {}

pub trait Stack {
    ///出栈
    fn pop() -> dyn Frame;

    ///入栈
    fn push(f: dyn Frame);

    ///获取当前栈顶指针
    fn top() -> &dyn Frame;

    ///获取当前栈底部指针
    fn bottom() -> &dyn Frame;

    ///栈的总大小
    fn size() -> usize;

    ///已使用栈的大小
    fn used() -> usize;

    ///剩余栈大小
    fn remain() -> usize;

    ///清理栈
    fn clean();
}

pub trait Coroutine {
    ///创建一个协程
    fn create(main: Option<dyn Coroutine>,
              stack: dyn Stack,
              init: usize,
              function: dyn FnOnce<dyn Any>,
              param_pointer: usize);

    ///将执行权交给另一个非主协程
    fn resume(coroutine: dyn Coroutine);

    ///非主协程将执行权交还给主协程
    fn yields();

    ///非主协程执行完毕
    fn exit();

    ///销毁协程
    fn destroy(coroutine: dyn Coroutine);

    ///获取协程的当前状态
    fn state();

    ///设置协程参数
    fn set_param(param_pointer: usize);

    ///获取协程参数
    fn get_param() -> usize;
}

///系统调用的hook
pub trait SystemCallHook {}

///调度器
pub trait Scheduler {}