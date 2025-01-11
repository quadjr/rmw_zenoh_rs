use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use zenoh::Wait;

use crate::rmw::rmw_qos_profile_t;
use crate::rmw::rmw_serialized_message_t;
use crate::EndpointInfo;
use crate::EntityType;
use crate::EventCallback;
use crate::EventMap;
use crate::GraphCache;
use crate::Node;
use crate::TypeSupport;

use crate::rmw::rmw_qos_history_policy_e_RMW_QOS_POLICY_HISTORY_KEEP_LAST as HISTORY_KEEP_LAST;

// The Endpoint struct represents a communication endpoint (e.g., Publisher, Subscriber)
pub struct Endpoint<T> {
    pub info: EndpointInfo,
    pub graph_cache: Arc<GraphCache>,
    pub sequence_number: AtomicI64,
    pub events: EventMap,
    pub message_buffer: Mutex<rmw_serialized_message_t>,
    pub send_type_support: Option<TypeSupport>,
    pub recv_type_support: Option<TypeSupport>,
    pub wait_set_cv: Arc<(Mutex<()>, Condvar)>,
    pub recv_fifo: Arc<Mutex<VecDeque<(i64, T)>>>,
    pub on_recv_callback: Arc<Mutex<EventCallback>>,
    #[allow(dead_code)]
    liveliness: zenoh::liveliness::LivelinessToken,
}

impl<T> Endpoint<T> {
    // Constructor for creating a new Endpoint
    pub fn new(
        node: &mut Node,
        entity_type: EntityType,
        endpoint_name: &str,
        send_type_support: Option<TypeSupport>,
        recv_type_support: Option<TypeSupport>,
        qos: rmw_qos_profile_t,
    ) -> Result<Self, ()> {
        // Clone the node's information and configure the endpoint
        let mut info = node.info.clone();
        info.entity_id = node.generate_entity_id();
        info.entity_type = entity_type;
        info.endpoint_name = endpoint_name.to_string();
        info.qos = qos;

        // Set the endpoint type based on type support
        if let Some(ref type_support) = send_type_support {
            info.endpoint_type = type_support.type_name.clone();
        } else if let Some(ref type_support) = recv_type_support {
            info.endpoint_type = type_support.type_name.clone();
        }

        // Create the endpoint instance
        let key_expr = info.to_string();
        let initial_capacity = info.qos.depth as usize;
        let endpoint = Endpoint {
            info,
            graph_cache: node.graph_cache.clone(),
            sequence_number: AtomicI64::new(1),
            events: Mutex::new(HashMap::new()),
            message_buffer: Mutex::new(
                rmw_serialized_message_t::new(0, node.context.allocator.clone()).map_err(|_| ())?,
            ),
            send_type_support,
            recv_type_support,
            wait_set_cv: node.context.wait_set_cv.clone(),
            recv_fifo: Arc::new(Mutex::new(VecDeque::with_capacity(initial_capacity))),
            on_recv_callback: Arc::new(Mutex::new((
                None::<unsafe extern "C" fn(*const ::std::os::raw::c_void, usize)>,
                0,
            ))),
            liveliness: node
                .context
                .session
                .liveliness()
                .declare_token(key_expr)
                .wait()
                .map_err(|_| ())?,
        };
        Ok(endpoint)
    }

    // Checks if the receive FIFO is empty
    pub fn is_empty(&self) -> bool {
        if let Ok(fifo) = self.recv_fifo.lock() {
            fifo.is_empty()
        } else {
            true
        }
    }

    // Pushes received data into the FIFO queue
    pub fn push_recv_data(&self, data: T) {
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(v) => v.as_nanos() as i64,
            Err(_) => 0,
        };

        if let Ok(mut fifo) = self.recv_fifo.lock() {
            // If QoS is keep-last and the queue is full, remove the oldest message
            if (self.info.qos.history == HISTORY_KEEP_LAST) && fifo.len() >= self.info.qos.depth {
                fifo.pop_front();
            }
            // Add the new message with a timestamp
            fifo.push_back((timestamp, data));
            // Notify waiting threads
            let (_, cvar) = &*self.wait_set_cv;
            cvar.notify_all();
        } else {
            return;
        }
        // Invoke the on-receive callback if it is set
        if let Ok(callback) = self.on_recv_callback.lock() {
            if let (Some(func), userdata) = *callback {
                unsafe {
                    func(
                        userdata as *const ::std::os::raw::c_void,
                        1, /* count is always 1 */
                    );
                }
            }
        }
    }

    // Takes a message from the FIFO queue
    pub fn take_message(&self) -> Option<(i64, T)> {
        if let Ok(mut fifo) = self.recv_fifo.lock() {
            fifo.pop_front()
        } else {
            None
        }
    }
}

// Clean up resources when the Endpoint is dropped
impl<T> Drop for Endpoint<T> {
    fn drop(&mut self) {
        if let Ok(mut buffer) = self.message_buffer.lock() {
            buffer.fini();
        }
    }
}
