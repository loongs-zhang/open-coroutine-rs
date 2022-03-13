#[link(name = "asm", kind = "static")]
extern "C" {
    /// todo
    pub fn swap(from: *mut Register, to: *const Register);
}

/// 此结构体直接映射到物理寄存器
/// 不同的CPU架构，其具体实现不同
/// 这里先只支持aarch64
pub(crate) struct Register {
    register: [usize; 32],
}

impl Register {
    pub(crate) fn new() -> Self {
        Register {
            register: [0; 32],
        }
    }
}