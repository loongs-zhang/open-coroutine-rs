use libc_x::co_crate;

fn main() {
    let _ = co_crate(2048, |param| {
        println!("Hello, world!");
        param
    }, None);
    //这里调用的实际上是被hook的实现，不是原始系统函数
    unsafe { libc::sleep(1); }
}
