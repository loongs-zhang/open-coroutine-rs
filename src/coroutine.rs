use std::any::Any;
use std::cell::{Ref, RefCell};
use std::rc::Rc;
use crate::api::{Coroutine, MainCoroutine, State};
use crate::Stack;

struct MainCoroutineImpl {}

impl MainCoroutineImpl {
    fn new() -> Self {
        MainCoroutineImpl {}
    }
}

impl MainCoroutine for MainCoroutineImpl {
    fn resume(&self, coroutine: Box<dyn Coroutine>) {
        todo!()
    }

    fn destroy(&self, coroutine: Box<dyn Coroutine>) {
        todo!()
    }

    fn exit(&self) {}
}

struct CoroutineImpl {
    /// 主协程
    main: Box<dyn MainCoroutine>,
    /// 寄存器
    register: [usize; 32],
    /// 独有栈
    stack: Box<dyn Stack>,
    /// 指向参数的指针
    param: Option<usize>,
    /// 协程状态
    state: Rc<RefCell<State>>,
    /// 函数指针
    function: Box<dyn FnOnce(dyn Any) -> ()>,
}

impl CoroutineImpl {
    fn new(main: Box<dyn MainCoroutine>,
           stack: Box<dyn Stack>,
           function: Box<dyn FnOnce(dyn Any) -> ()>,
           param_pointer: Option<usize>) -> Self {
        CoroutineImpl {
            main,
            register: [0; 32],
            stack,
            param: param_pointer,
            state: Rc::new(RefCell::new(State::Created)),
            function,
        }
    }
}

impl Coroutine for CoroutineImpl {
    fn yields(&self) {
        todo!()
    }

    fn exit(&self) {
        self.yields();
        *self.state.borrow_mut() = State::Exited;
    }

    fn get_state(&self) -> Ref<State> {
        return self.state.borrow();
    }

    fn get_main_coroutine(&self) -> &dyn MainCoroutine {
        &*self.main
    }

    fn set_param(&mut self, param_pointer: usize) {
        self.param = Some(param_pointer);
    }

    fn get_param(&self) -> Option<usize> {
        self.param
    }
}