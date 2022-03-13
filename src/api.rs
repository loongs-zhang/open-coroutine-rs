use std::cell::Ref;
use crate::register::Register;

#[derive(PartialEq, Copy, Clone)]
pub enum State {
    /// 已创建
    Created,
    /// 运行中
    Running,
    /// 被挂起
    Suspend,
    /// 已退出
    Exited,
}

pub trait MainCoroutine {
    /// 获取物理寄存器
    fn get_register(&self) -> &Register;

    /// 将执行权交给另一个非主协程
    fn resume(&self, coroutine: Box<dyn Coroutine>);

    /// 销毁协程
    fn destroy(&self, coroutine: Box<dyn Coroutine>);

    /// 主协程执行完毕
    fn exit(&self);
}

pub trait Coroutine {
    /// 获取物理寄存器
    fn get_register(&self) -> &Register;

    /// 非主协程将执行权交还给主协程
    fn yields(&self);

    /// 非主协程执行完毕
    fn exit(&self);

    /// 获取协程的当前状态
    fn get_state(&self) -> State;

    /// 获取主协程
    fn get_main_coroutine(&self) -> &dyn MainCoroutine;

    /// 设置协程参数
    fn set_param(&mut self, param_pointer: usize);

    /// 获取协程参数
    fn get_param(&self) -> Option<usize>;
}