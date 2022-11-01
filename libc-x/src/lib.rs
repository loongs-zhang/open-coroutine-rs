use open_coroutine::coroutine::Coroutine;
use std::os::raw::c_void;

extern "C" {
    fn coroutine_crate(coroutine: *mut c_void);
}

pub fn co_crate<F>(size: usize, proc: F, param: Option<*mut c_void>) -> Coroutine<F>
where
    F: FnOnce(Option<*mut c_void>) -> Option<*mut c_void> + Sized,
{
    let mut co = Coroutine::new(size, proc, param);
    unsafe {
        coroutine_crate(&mut co as *mut _ as *mut c_void);
    }
    co
}

#[cfg(test)]
mod tests {
    use crate::co_crate;

    #[test]
    fn test_sleep() {
        unsafe {
            let x = 10;
            let _co = co_crate(
                2048,
                |param| {
                    println!("hello from coroutine {}", x);
                    param
                },
                None,
            );
            libc::sleep(1);
        }
    }
}
