use std::sync::atomic::Ordering;
use std::sync::Arc;
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

pub struct Client<'a> {
    client: zenoh::query::Querier<'a>,
    pub endpoint: Arc<Endpoint<Reply>>,
}

impl<'a> Client<'a> {
    pub fn new(
        node: &mut Node,
        endpoint_name: &str,
        request_type_support: TypeSupport,
        response_type_support: TypeSupport,
        mut qos: rmw_qos_profile_t,
    ) -> Result<Self, ()> {
        qos.set_default_profile();
        let endpoint = Arc::new(Endpoint::new(
            node,
            EntityType::Client,
            endpoint_name,
            Some(request_type_support),
            Some(response_type_support),
            qos,
        )?);
        let key_expr = endpoint.info.get_endpoint_keyexpr();
        let client = node
            .context
            .session
            .declare_querier(key_expr)
            .wait()
            .map_err(|_| ())?;
        Ok(Client { client, endpoint })
    }

    pub fn send_request(&self, ros_request: *const ::std::os::raw::c_void) -> Result<i64, ()> {
        let mut msg = self.endpoint.message_buffer.lock().map_err(|_| ())?;
        let type_support = self.endpoint.send_type_support.as_ref().ok_or(())?;
        type_support.serialize(ros_request, &mut *msg)?;
        let endpoint = self.endpoint.clone();

        let seq = self
            .endpoint
            .sequence_number
            .fetch_add(1, Ordering::Relaxed);

        let attachment: ZBytes = Attachment::new(
            seq,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |v| v.as_nanos() as i64),
            self.endpoint.info.get_gid(),
        )
        .try_into()?;

        let payload = unsafe { std::slice::from_raw_parts(msg.buffer, msg.buffer_length) };
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

    pub fn take_response(
        &self,
        request_header: *mut rmw_service_info_t,
        ros_response: *mut ::std::os::raw::c_void,
    ) -> Result<bool, ()> {
        let Some(data) = self.endpoint.take_message() else {
            return Ok(false);
        };

        let mut msg = self.endpoint.message_buffer.lock().map_err(|_| ())?;
        let type_support = self.endpoint.recv_type_support.as_ref().ok_or(())?;
        let result = data.1.result().map_err(|_| ())?;
        read_payload(result.payload(), &mut msg)?;
        type_support.deserialize(&*msg, ros_response)?;

        let request_header = unsafe { &mut *request_header };
        request_header.received_timestamp = data.0;

        let attachment: Attachment = result.attachment().ok_or(())?.try_into()?;
        request_header.source_timestamp = attachment.source_timestamp;
        request_header.request_id.sequence_number = attachment.sequence_number;
        request_header.request_id.writer_guid = attachment.source_gid;
        Ok(true)
    }
}

impl<'a> WaitSetTrait for Client<'a> {
    fn is_empty(&self) -> bool {
        self.endpoint.is_empty()
    }
}
