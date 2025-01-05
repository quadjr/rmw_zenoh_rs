use crate::WaitSetTrait;
use std::sync::{Arc, Condvar, Mutex};

pub struct GuardCondition {
    triggered: bool,
    pub wait_set_cv: Arc<(Mutex<()>, Condvar)>,
}

impl GuardCondition {
    pub fn new(wait_set_cv: Arc<(Mutex<()>, Condvar)>) -> Self {
        GuardCondition {
            triggered: false,
            wait_set_cv,
        }
    }

    pub fn trigger(&mut self) {
        let (lock, cvar) = &*self.wait_set_cv;
        if let Ok(_) = lock.lock() {
            self.triggered = true;
            cvar.notify_all();
        }
    }
}

impl WaitSetTrait for GuardCondition {
    fn is_empty(&self) -> bool {
        !self.triggered
    }
    fn cleanup(&mut self) {
        self.triggered = false;
    }
}
