use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use zenoh::bytes::ZBytes;
use zenoh::Wait;
use zenoh_ext::{AdvancedPublisher, AdvancedPublisherBuilderExt, CacheConfig};

use crate::rmw::rmw_qos_durability_policy_e_RMW_QOS_POLICY_DURABILITY_TRANSIENT_LOCAL as DURABILITY_TRANSIENT_LOCAL;
use crate::rmw::rmw_qos_profile_t;
use crate::rmw::rmw_serialized_message_t;
use crate::Attachment;
use crate::Endpoint;
use crate::EntityType;
use crate::Node;
use crate::TypeSupport;

enum PublisherEnum<'a> {
    Publisher(zenoh::pubsub::Publisher<'a>),
    AdvancedPublisher(AdvancedPublisher<'a>),
}

pub struct Publisher<'a> {
    publisher: PublisherEnum<'a>,
    pub endpoint: Arc<Endpoint<()>>,
}

impl<'a> Publisher<'a> {
    pub fn new(
        node: &mut Node<'a>,
        endpoint_name: &str,
        type_support: TypeSupport,
        mut qos: rmw_qos_profile_t,
    ) -> Result<Self, ()> {
        qos.set_default_profile();
        let endpoint = Arc::new(Endpoint::new(
            node,
            EntityType::Publisher,
            endpoint_name,
            Some(type_support),
            None,
            qos,
        )?);
        let key_expr = endpoint.info.get_endpoint_keyexpr();
        if qos.durability == DURABILITY_TRANSIENT_LOCAL {
            Ok(Publisher {
                publisher: PublisherEnum::AdvancedPublisher(
                    node.context
                        .session
                        .declare_publisher(key_expr)
                        .cache(CacheConfig::default().max_samples(qos.depth))
                        .wait()
                        .map_err(|_| ())?,
                ),
                endpoint,
            })
        } else {
            Ok(Publisher {
                publisher: PublisherEnum::Publisher(
                    node.context
                        .session
                        .declare_publisher(key_expr)
                        .wait()
                        .map_err(|_| ())?,
                ),
                endpoint,
            })
        }
    }

    pub fn publish(&self, ros_message: *const ::std::os::raw::c_void) -> Result<(), ()> {
        let mut msg = self.endpoint.message_buffer.lock().map_err(|_| ())?;
        let type_support = self.endpoint.send_type_support.as_ref().ok_or(())?;
        type_support.serialize(ros_message, &mut *msg)?;
        self.publish_serialized_message(&*msg)
    }

    pub fn publish_serialized_message(&self, msg: &rmw_serialized_message_t) -> Result<(), ()> {
        let attachment: ZBytes = Attachment::new(
            self.endpoint
                .sequence_number
                .fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |v| v.as_nanos() as i64),
            self.endpoint.info.get_gid(),
        )
        .try_into()?;

        let payload = unsafe { std::slice::from_raw_parts(msg.buffer, msg.buffer_length) };

        match &self.publisher {
            PublisherEnum::Publisher(publisher) => publisher
                .put(payload)
                .attachment(attachment)
                .wait()
                .map_or_else(|_| Err(()), |_| Ok(())),
            PublisherEnum::AdvancedPublisher(publisher) => publisher
                .put(payload)
                .attachment(attachment)
                .wait()
                .map_or_else(|_| Err(()), |_| Ok(())),
        }
    }
}
