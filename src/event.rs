use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use crate::rmw::rmw_event_callback_t;
use crate::rmw::rmw_event_type_t;
use crate::WaitSetTrait;

pub type EventMap = Mutex<HashMap<EventType, Box<Event>>>;
pub type EventCallback = (rmw_event_callback_t, usize);
pub type EventType = rmw_event_type_t;

pub struct Event {
    pub _event_type: EventType,
    pub event_queue: VecDeque<()>,
    pub event_callback: Option<EventCallback>,
}

impl Event {
    pub fn new(_event_type: EventType) -> Self {
        Event {
            _event_type,
            event_queue: VecDeque::new(),
            event_callback: None,
        }
    }
}

impl WaitSetTrait for Event {
    fn is_empty(&self) -> bool {
        self.event_queue.is_empty()
    }
}
