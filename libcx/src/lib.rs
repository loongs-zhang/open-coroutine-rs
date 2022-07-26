#[cfg(test)]
mod tests {
    use open_coroutine::coroutine::Coroutine;
    use open_coroutine_scheduler::Scheduler;

    #[test]
    fn test_sleep() {
        //fixme 这里的协程不会执行
        Scheduler::global().execute(Coroutine::new(2048, |param| {
            println!("test_sleep execute when sleep");
            param
        }, None));
        println!("test_sleep ready {}",Scheduler::global().get_ready().len());
        unsafe {
            libc::sleep(1);
        }
        Scheduler::global().schedule();
        println!("finished");
    }
}