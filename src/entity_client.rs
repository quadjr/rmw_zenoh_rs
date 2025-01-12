use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use zenoh::bytes::ZBytes;
use zenoh::query::Reply;
use zenoh::Wait;

use crate::read_payload;
use crate::rmw::rmw_qos_profile_t;
use crate::rmw::rmw_service_info_t;
use crate::Attachment;
use crate::Endpoint;
use crate::EntityType;
use crate::Node;
use crate::TypeSupport;
use crate::WaitSetTrait;

// Client struct: Represents a ROS 2 client entity
pub struct Client<'a> {
    client: zenoh::query::Querier<'a>,
    pub endpoint: Arc<Endpoint<Reply>>,
}

impl<'a> Client<'a> {
    // Constructor for creating a new Client instance
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
            EntityType::Client,
            endpoint_name,
            Some(request_type_support),
            Some(response_type_support),
            qos,
        )?);
        // Generate the key expression for the endpoint
        let key_expr = endpoint.info.get_endpoint_keyexpr();
        let client = node
            .context
            .session
            .declare_querier(key_expr)
            .timeout(Duration::MAX)
            .wait()
            .map_err(|_| ())?;
        Ok(Client { client, endpoint })
    }

    // Sends a request to the service
    pub fn send_request(&self, ros_request: *const ::std::os::raw::c_void) -> Result<i64, ()> {
        // Serialize Message
        let mut msg = self.endpoint.message_buffer.lock().map_err(|_| ())?;
        let type_support = self.endpoint.send_type_support.as_ref().ok_or(())?;
        type_support.serialize(ros_request, &mut *msg)?;
        // Increment the sequence number
        let seq = self
            .endpoint
            .sequence_number
            .fetch_add(1, Ordering::Relaxed);
        // Create an attachment with metadata
        let attachment: ZBytes = Attachment::new(
            seq,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |v| v.as_nanos() as i64),
            self.endpoint.info.get_gid(),
        )
        .try_into()?;
        // Send request
        let payload = unsafe { std::slice::from_raw_parts(msg.buffer, msg.buffer_length) };
        let endpoint = self.endpoint.clone();
        self.client
            .get()
            .payload(payload)
            .attachment(attachment)
            .callback(move |reply| {
                endpoint.push_recv_data(reply);
            })
            .wait()
            .map_err(|_| ())?;
        Ok(seq)
    }
    // Takes a response from the service
    pub fn take_response(
        &self,
        request_header: *mut rmw_service_info_t,
        ros_response: *mut ::std::os::raw::c_void,
    ) -> Result<bool, ()> {
        // Attempt to take a message from the endpoint
        let Some(data) = self.endpoint.take_message() else {
            // Return false if no message is available
            return Ok(false);
        };
        // Deserialize the response into the ROS message
        let mut msg = self.endpoint.message_buffer.lock().map_err(|_| ())?;
        let type_support = self.endpoint.recv_type_support.as_ref().ok_or(())?;
        let result = data.1.result().map_err(|_| ())?;
        read_payload(result.payload(), &mut msg)?;
        type_support.deserialize(&*msg, ros_response)?;
        // Set the received timestamp
        let request_header = unsafe { &mut *request_header };
        request_header.received_timestamp = data.0;
        // Parse the attachment
        let attachment: Attachment = result.attachment().ok_or(())?.try_into()?;
        request_header.source_timestamp = attachment.source_timestamp;
        request_header.request_id.sequence_number = attachment.sequence_number;
        request_header.request_id.writer_guid = attachment.source_gid;
        Ok(true)
    }
}

// Implements WaitSetTrait for the Client
impl<'a> WaitSetTrait for Client<'a> {
    fn is_empty(&self) -> bool {
        self.endpoint.is_empty()
    }
}
