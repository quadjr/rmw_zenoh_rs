use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use crate::rmw::rmw_event_callback_t;
use crate::rmw::rmw_event_type_t;
use crate::WaitSetTrait;

// Type aliases for better readability
pub type EventMap = Mutex<HashMap<rmw_event_type_t, Box<Event>>>;
pub type EventCallback = (rmw_event_callback_t, usize);

// The `Event` struct represents an individual event in the system.
pub struct Event {
    pub _event_type: rmw_event_type_t,
    pub event_queue: VecDeque<()>,
    pub event_callback: Option<EventCallback>,
}

impl Event {
    // Constructor for creating a new Event instance
    pub fn new(_event_type: rmw_event_type_t) -> Self {
        Event {
            _event_type,
            event_queue: VecDeque::new(),
            event_callback: None,
        }
    }
}

// Implements WaitSetTrait for the Event
impl WaitSetTrait for Event {
    fn is_empty(&self) -> bool {
        self.event_queue.is_empty()
    }
}
