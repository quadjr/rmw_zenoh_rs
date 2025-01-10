use std::sync::Arc;
use zenoh::sample::Sample;
use zenoh::Wait;
use zenoh_ext::{AdvancedSubscriber, AdvancedSubscriberBuilderExt, HistoryConfig};

use crate::read_payload;
use crate::rmw::rmw_message_info_t;
use crate::rmw::rmw_qos_durability_policy_e_RMW_QOS_POLICY_DURABILITY_TRANSIENT_LOCAL as DURABILITY_TRANSIENT_LOCAL;
use crate::rmw::rmw_qos_profile_t;
use crate::rmw::rmw_serialized_message_t;
use crate::Attachment;
use crate::Endpoint;
use crate::EntityType;
use crate::Node;
use crate::TypeSupport;
use crate::WaitSetTrait;
use crate::IMPLEMENTATION_IDENTIFIER_CHAR;
use crate::RMW_GID_STORAGE_SIZE;

// Enum to represent two types of Zenoh subscriber
enum SubscriberEnum {
    #[allow(dead_code)]
    Subscriber(zenoh::pubsub::Subscriber<()>),
    #[allow(dead_code)]
    AdvancedSubscriber(AdvancedSubscriber<()>),
}

// Subscriber struct: Represents a ROS 2 subscriber entity
pub struct Subscriber {
    #[allow(dead_code)]
    subscriber: SubscriberEnum,
    pub endpoint: Arc<Endpoint<Sample>>,
}

impl Subscriber {
    // Constructor for creating a new Subscriber instance
    pub fn new(
        node: &mut Node,
        endpoint_name: &str,
        type_support: TypeSupport,
        mut qos: rmw_qos_profile_t,
        ignore_local_publications: bool,
    ) -> Result<Self, ()> {
        // Ensure default QoS settings are applied
        qos.set_default_profile();
        let endpoint = Arc::new(Endpoint::new(
            node,
            EntityType::Subscriber,
            endpoint_name,
            None,
            Some(type_support),
            qos,
        )?);
        // Generate the key expression for the endpoint
        let key_expr = endpoint.info.get_subscriber_keyexpr();
        let local_publisher_key_expr = endpoint.info.get_publisher_keyexpr();
        let endpoint_clone = endpoint.clone();
        // Check if durability is set to Transient Local
        if qos.durability == DURABILITY_TRANSIENT_LOCAL {
            // Create an advanced subscriber with caching
            Ok(Subscriber {
                subscriber: SubscriberEnum::AdvancedSubscriber(
                    node.context
                        .session
                        .declare_subscriber(key_expr)
                        .history(HistoryConfig::default().detect_late_publishers())
                        .callback(move |sample| {
                            if !ignore_local_publications
                                || sample.key_expr().as_str() != local_publisher_key_expr
                            {
                                endpoint_clone.push_recv_data(sample);
                            }
                        })
                        .wait()
                        .map_err(|_| ())?,
                ),
                endpoint,
            })
        } else {
            // Create a standard subscriber without caching
            Ok(Subscriber {
                subscriber: SubscriberEnum::Subscriber(
                    node.context
                        .session
                        .declare_subscriber(key_expr)
                        .callback(move |sample| {
                            if !ignore_local_publications
                                || sample.key_expr().as_str() != local_publisher_key_expr
                            {
                                endpoint_clone.push_recv_data(sample);
                            }
                        })
                        .wait()
                        .map_err(|_| ())?,
                ),
                endpoint,
            })
        }
    }
    // Takes a deserialized ROS message and its metadata
    pub fn take_message(
        &self,
        ros_message: *mut ::std::os::raw::c_void,
        message_info: *mut rmw_message_info_t,
    ) -> Result<bool, ()> {
        // Take the serialized message
        let mut msg = self.endpoint.message_buffer.lock().map_err(|_| ())?;
        let taken = self.take_serialized_message(&mut *msg, message_info)?;
        if taken {
            // Deserialize the message
            let type_support = self.endpoint.recv_type_support.as_ref().ok_or(())?;
            type_support.deserialize(&*msg, ros_message)?
        }
        Ok(taken)
    }
    // Takes a serialized message and its metadata
    pub fn take_serialized_message(
        &self,
        serialized_message: &mut rmw_serialized_message_t,
        message_info: *mut rmw_message_info_t,
    ) -> Result<bool, ()> {
        // Attempt to take a message from the endpoint
        let Some(data) = self.endpoint.take_message() else {
            return Ok(false);
        };
        // Read the payload into the serialized message buffer
        read_payload(data.1.payload(), serialized_message)?;
        // Fill in the message metadata
        if !message_info.is_null() {
            // Parse the attachment
            let attachment: Attachment = data.1.attachment().ok_or(())?.try_into()?;
            let info = unsafe { &mut *message_info };
            info.source_timestamp = attachment.source_timestamp;
            info.publication_sequence_number = attachment.sequence_number as u64;
            info.publisher_gid.implementation_identifier = IMPLEMENTATION_IDENTIFIER_CHAR;
            info.publisher_gid.data = [0; RMW_GID_STORAGE_SIZE as usize];
            for i in 0..attachment.source_gid.len() {
                info.publisher_gid.data[i] = attachment.source_gid[i] as u8;
            }
            info.received_timestamp = data.0;
            info.reception_sequence_number = u64::MAX;
            info.from_intra_process = false;
        }
        Ok(true)
    }
}

// Implements WaitSetTrait for the Subscriber
impl WaitSetTrait for Subscriber {
    fn is_empty(&self) -> bool {
        self.endpoint.is_empty()
    }
}
