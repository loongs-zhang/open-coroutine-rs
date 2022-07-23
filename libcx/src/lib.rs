#[cfg(test)]
mod tests {
    #[test]
    fn test_sleep() {
        unsafe {
            libc::sleep(1);
        }
        println!("finished");
    }
}