use std::os::raw::c_void;

extern "C" {
    pub fn coroutine_crate(coroutine: *mut c_void);
}

#[cfg(test)]
mod tests {
    use open_coroutine::coroutine::Coroutine;
    use crate::coroutine_crate;

    #[test]
    fn test_sleep() {
        unsafe {
            let x = 10;
            let mut co = Coroutine::new(2048, |param| {
                println!("hello from coroutine {}", x);
                param
            }, None);
            coroutine_crate(std::mem::transmute(&mut co));
            libc::sleep(1);
        }
    }
}