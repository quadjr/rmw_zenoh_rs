use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use zenoh::bytes::ZBytes;
use zenoh::query::Query;
use zenoh::Wait;

use crate::read_payload;
use crate::rmw::rmw_qos_profile_t;
use crate::rmw::rmw_request_id_t;
use crate::rmw::rmw_service_info_t;
use crate::Attachment;
use crate::Endpoint;
use crate::EntityType;
use crate::Node;
use crate::TypeSupport;
use crate::WaitSetTrait;

// Extension of `rmw_request_id_t` to include hashing functionality
impl rmw_request_id_t {
    pub fn get_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.writer_guid.hash(&mut hasher);
        self.sequence_number.hash(&mut hasher);
        hasher.finish()
    }
}

// Service struct: Represents a ROS 2 service entity
pub struct Service {
    #[allow(dead_code)]
    service: zenoh::query::Queryable<()>,
    query_map: Arc<Mutex<HashMap<u64, zenoh::query::Query>>>,
    pub endpoint: Arc<Endpoint<Query>>,
}

impl Service {
    // Constructor for creating a new Service instance
    pub fn new(
        node: &mut Node,
        endpoint_name: &str,
        request_type_support: TypeSupport,
        response_type_support: TypeSupport,
        mut qos: rmw_qos_profile_t,
    ) -> Result<Self, ()> {
        // Ensure default QoS settings are applied
        qos.set_default_profile();
        let endpoint = Arc::new(Endpoint::new(
            node,
            EntityType::Service,
            endpoint_name,
            Some(response_type_support),
            Some(request_type_support),
            qos,
        )?);
        // Generate the key expression for the endpoint
        let key_expr = endpoint.info.get_endpoint_keyexpr();
        let endpoint_clone = endpoint.clone();
        let service = node
            .context
            .session
            .declare_queryable(key_expr)
            .callback(move |query| {
                endpoint_clone.push_recv_data(query);
            })
            .wait()
            .map_err(|_| ())?;
        Ok(Service {
            service,
            query_map: Arc::new(Mutex::new(HashMap::new())),
            endpoint,
        })
    }

    // Sends a response to the client
    pub fn send_response(
        &self,
        request_header: *mut rmw_request_id_t,
        ros_response: *mut ::std::os::raw::c_void,
    ) -> Result<(), ()> {
        // Serialize Message
        let mut msg = self.endpoint.message_buffer.lock().map_err(|_| ())?;
        let type_support = self.endpoint.send_type_support.as_ref().ok_or(())?;
        type_support.serialize(ros_response, &mut *msg)?;
        // Retrieve the query associated with the request
        let request_header = unsafe { &*request_header };
        let mut map = self.query_map.lock().map_err(|_| ())?;
        let query = map.remove(&request_header.get_hash()).ok_or(())?;
        // Create an attachment with metadata
        let attachment: ZBytes = Attachment::new(
            request_header.sequence_number,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |v| v.as_nanos() as i64),
            self.endpoint.info.get_gid(),
        )
        .try_into()?;
        // Send response
        let payload = unsafe { std::slice::from_raw_parts(msg.buffer, msg.buffer_length) };
        query
            .reply(query.key_expr(), payload)
            .attachment(attachment)
            .wait()
            .map_or_else(|_| Err(()), |_| Ok(()))
    }
    // Takes a request from the client
    pub fn take_request(
        &self,
        request_header: *mut rmw_service_info_t,
        ros_request: *mut ::std::os::raw::c_void,
    ) -> Result<bool, ()> {
        // Attempt to take a message from the endpoint
        let Some(data) = self.endpoint.take_message() else {
            return Ok(false);
        };
        // Deserialize the response into the ROS message
        let mut msg = self.endpoint.message_buffer.lock().map_err(|_| ())?;
        let type_support = self.endpoint.recv_type_support.as_ref().ok_or(())?;
        let payload = data.1.payload().ok_or(())?;
        read_payload(payload, &mut msg)?;
        type_support.deserialize(&*msg, ros_request)?;
        // Parse the attachment
        let attachment: Attachment = data.1.attachment().ok_or(())?.try_into()?;
        let request_header = unsafe { &mut *request_header };
        request_header.source_timestamp = attachment.source_timestamp;
        request_header.request_id.sequence_number = attachment.sequence_number;
        request_header.request_id.writer_guid = attachment.source_gid;
        // Insert the request into the map
        let mut map = self.query_map.lock().map_err(|_| ())?;
        map.insert(request_header.request_id.get_hash(), data.1);
        Ok(true)
    }
}

// Implements WaitSetTrait for the Service
impl WaitSetTrait for Service {
    fn is_empty(&self) -> bool {
        self.endpoint.is_empty()
    }
}
