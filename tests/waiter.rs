#[cfg(test)]
mod test {
    use simplep2pgossip::waiter::Waiter;
    use std::thread;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_waiter() -> Result<(), String> {
        let wt = Waiter::new();
        let wt_copy = wt.clone();
        let checker = Arc::new(Mutex::new(false));
        let checker_copy = checker.clone();
        let handle = thread::spawn(move || {
            assert!(wt_copy.wait().is_ok());
            assert!(checker_copy.lock().and_then(move |mut val_storage| { *val_storage = true; Ok(()) }).is_ok())
        });
        assert!(wt.broadcast().is_ok());
        assert!(handle.join().is_ok());
        assert!(*checker.lock().map_err(|err| {format!("Error on lock: {:?}", err)})?);

        Ok(())
    }

    #[test]
    fn test_no_wait() -> Result<(), String> {
        let wt = Waiter::new();
        let wt_copy = wt.clone();
        let checker = Arc::new(Mutex::new(false));
        let checker_copy = checker.clone();

        assert!(wt.broadcast().is_ok());
        let handle = thread::spawn(move || {
            assert!(wt_copy.wait().is_ok());
            assert!(checker_copy.lock().and_then(move |mut val_storage| { *val_storage = true; Ok(()) }).is_ok())
        });
        assert!(handle.join().is_ok());
        assert!(*checker.lock().map_err(|err| {format!("Error on lock: {:?}", err)})?);
        Ok(())
    }
}