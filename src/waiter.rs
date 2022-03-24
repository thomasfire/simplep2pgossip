
use std::sync::{Condvar, Mutex, Arc, LockResult, MutexGuard};
use log::error;

#[derive(Debug, Clone)]
pub struct Waiter {
    mtx: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
}

impl Waiter {
    pub fn new() -> Self {
        Waiter {mtx: Arc::new(Mutex::new(false)), cv: Arc::new(Condvar::new())}
    }
    pub fn wait(&self) -> Result<(), ()> {
        let mut started = self.mtx.lock().map_err(|err| {error!("Poisoned mutex: {:?}", err);()})?;
        while !*started {
            started = self.cv.wait(started).unwrap();
        }
        *started = false;
        Ok(())
    }
    pub fn broadcast(&self) -> Result<(), ()> {
        let mut started = self.mtx.lock().map_err(|err| {error!("Poisoned mutex: {:?}", err);()})?;
        *started = true;
        self.cv.notify_all();
        Ok(())
    }
}