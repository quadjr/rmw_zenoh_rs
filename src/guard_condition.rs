use crate::WaitSetTrait;
use std::sync::{Arc, Condvar, Mutex};

// Represents a guard condition used to notify wait sets.
pub struct GuardCondition {
    triggered: bool,
    pub wait_set_cv: Arc<(Mutex<()>, Condvar)>,
}

impl GuardCondition {
    // Constructor for creating a new GuardCondition instance
    pub fn new(wait_set_cv: Arc<(Mutex<()>, Condvar)>) -> Self {
        GuardCondition {
            triggered: false,
            wait_set_cv,
        }
    }
    // Triggers the guard condition, notifying all waiting threads.
    pub fn trigger(&mut self) {
        let (lock, cvar) = &*self.wait_set_cv;
        if let Ok(_) = lock.lock() {
            self.triggered = true;
            cvar.notify_all();
        }
    }
}

// Implements WaitSetTrait for the GuardCondition
impl WaitSetTrait for GuardCondition {
    fn is_empty(&self) -> bool {
        !self.triggered
    }
    fn cleanup(&mut self) {
        self.triggered = false;
    }
}
