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

enum SubscriberEnum {
    #[allow(dead_code)]
    Subscriber(zenoh::pubsub::Subscriber<()>),
    #[allow(dead_code)]
    AdvancedSubscriber(AdvancedSubscriber<()>),
}

pub struct Subscriber {
    #[allow(dead_code)]
    subscriber: SubscriberEnum,
    pub endpoint: Arc<Endpoint<Sample>>,
}

impl Subscriber {
    pub fn new(
        node: &mut Node,
        endpoint_name: &str,
        type_support: TypeSupport,
        mut qos: rmw_qos_profile_t,
    ) -> Result<Self, ()> {
        qos.set_default_profile();
        let endpoint = Arc::new(Endpoint::new(
            node,
            EntityType::Subscriber,
            endpoint_name,
            None,
            Some(type_support),
            qos,
        )?);
        let key_expr = endpoint.info.get_endpoint_keyexpr();
        let endpoint_clone = endpoint.clone();
        if qos.durability == DURABILITY_TRANSIENT_LOCAL {
            Ok(Subscriber {
                subscriber: SubscriberEnum::AdvancedSubscriber(
                    node.context
                        .session
                        .declare_subscriber(key_expr)
                        .history(HistoryConfig::default().detect_late_publishers())
                        .callback(move |sample| {
                            endpoint_clone.push_recv_data(sample);
                        })
                        .wait()
                        .map_err(|_| ())?,
                ),
                endpoint,
            })
        } else {
            Ok(Subscriber {
                subscriber: SubscriberEnum::Subscriber(
                    node.context
                        .session
                        .declare_subscriber(key_expr)
                        .callback(move |sample| {
                            endpoint_clone.push_recv_data(sample);
                        })
                        .wait()
                        .map_err(|_| ())?,
                ),
                endpoint,
            })
        }
    }

    pub fn take_message(
        &self,
        ros_message: *mut ::std::os::raw::c_void,
        message_info: *mut rmw_message_info_t,
    ) -> Result<bool, ()> {
        let mut msg = self.endpoint.message_buffer.lock().map_err(|_| ())?;
        let taken = self.take_serialized_message(&mut *msg, message_info)?;
        if taken {
            let type_support = self.endpoint.recv_type_support.as_ref().ok_or(())?;
            type_support.deserialize(&*msg, ros_message)?
        }
        Ok(taken)
    }

    pub fn take_serialized_message(
        &self,
        serialized_message: &mut rmw_serialized_message_t,
        message_info: *mut rmw_message_info_t,
    ) -> Result<bool, ()> {
        // Read payload
        let Some(data) = self.endpoint.take_message() else {
            return Ok(false);
        };
        read_payload(data.1.payload(), serialized_message)?;

        // Fill message_info
        if message_info.is_null() {
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

impl WaitSetTrait for Subscriber {
    fn is_empty(&self) -> bool {
        self.endpoint.is_empty()
    }
}
