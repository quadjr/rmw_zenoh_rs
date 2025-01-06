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

// Enum to represent two types of Zenoh publishers
enum PublisherEnum<'a> {
    Publisher(zenoh::pubsub::Publisher<'a>),
    AdvancedPublisher(AdvancedPublisher<'a>),
}

// Publisher struct: Represents a ROS 2 publisher entity
pub struct Publisher<'a> {
    publisher: PublisherEnum<'a>,
    pub endpoint: Arc<Endpoint<()>>,
}

impl<'a> Publisher<'a> {
    // Constructor for creating a new Publisher instance
    pub fn new(
        node: &mut Node<'a>,
        endpoint_name: &str,
        type_support: TypeSupport,
        mut qos: rmw_qos_profile_t,
    ) -> Result<Self, ()> {
        // Ensure default QoS settings are applied
        qos.set_default_profile();
        let endpoint = Arc::new(Endpoint::new(
            node,
            EntityType::Publisher,
            endpoint_name,
            Some(type_support),
            None,
            qos,
        )?);
        // Generate the key expression for the endpoint
        let key_expr = endpoint.info.get_endpoint_keyexpr();
        // Check if durability is set to Transient Local
        if qos.durability == DURABILITY_TRANSIENT_LOCAL {
            // Create an advanced publisher with caching
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
            // Create a standard publisher without caching
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
    // Publishes a ROS message
    pub fn publish(&self, ros_message: *const ::std::os::raw::c_void) -> Result<(), ()> {
        // Serialize the ROS message
        let mut msg = self.endpoint.message_buffer.lock().map_err(|_| ())?;
        let type_support = self.endpoint.send_type_support.as_ref().ok_or(())?;
        type_support.serialize(ros_message, &mut *msg)?;
        // Publish the serialized message
        self.publish_serialized_message(&*msg)
    }
    // Publishes a serialized message
    pub fn publish_serialized_message(&self, msg: &rmw_serialized_message_t) -> Result<(), ()> {
        // Create an attachment with metadata
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

        // Publish the message using the appropriate publisher
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
