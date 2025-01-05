mod context;
mod endpoint;
mod endpoint_info;
mod entity_client;
mod entity_node;
mod entity_publisher;
mod entity_service;
mod entity_subscriber;
mod entity_utils;
mod event;
mod graph_cache;
mod graph_cache_utils;
mod guard_condition;
mod qos;
pub mod rmw;
pub mod rsutils;
mod type_support;

use context::Context;
use endpoint::Endpoint;
use endpoint_info::EndpointInfo;
use endpoint_info::EntityType;
use entity_client::Client;
use entity_node::Node;
use entity_publisher::Publisher;
use entity_service::Service;
use entity_subscriber::Subscriber;
use entity_utils::read_payload;
use entity_utils::Attachment;
use entity_utils::WaitSetTrait;
use event::Event;
use event::EventCallback;
use event::EventMap;
use graph_cache::GraphCache;
use guard_condition::GuardCondition;
use rmw::RMW_GID_STORAGE_SIZE;
use rsutils::StringStorage;
use type_support::TypeSupport;

const RMW_GID_STORAGE_SIZE_IRON: usize = 16;
const ADMIN_SPACE: &str = "@ros2_lv";
const IMPLEMENTATION_IDENTIFIER_STR: &str = "rmw_zenoh_rs";
const IMPLEMENTATION_IDENTIFIER_CHAR: *const ::std::os::raw::c_char =
    "rmw_zenoh_rs\0".as_ptr() as *const ::std::os::raw::c_char;
const SERIALIZATION_FORMAT_CHAR: *const ::std::os::raw::c_char =
    "cdr\0".as_ptr() as *const ::std::os::raw::c_char;

use crate::rmw::rmw_qos_durability_policy_e_RMW_QOS_POLICY_DURABILITY_TRANSIENT_LOCAL as DURABILITY_TRANSIENT_LOCAL;
use crate::rmw::rmw_qos_history_policy_e_RMW_QOS_POLICY_HISTORY_KEEP_LAST as HISTORY_KEEP_LAST;
use crate::rmw::rmw_qos_liveliness_policy_e_RMW_QOS_POLICY_LIVELINESS_AUTOMATIC as POLICY_LIVELINESS_AUTOMATIC;
use crate::rmw::rmw_qos_reliability_policy_e_RMW_QOS_POLICY_RELIABILITY_RELIABLE as ELIABILITY_RELIABLE;
const RMW_DURATION_INFINITE: rmw::rmw_time_t = rmw::rmw_time_t {
    sec: 9223372036,
    nsec: 854775807,
};
const DEFAULT_QOS: rmw::rmw_qos_profile_t = rmw::rmw_qos_profile_t {
    history: HISTORY_KEEP_LAST,
    depth: 10,
    reliability: ELIABILITY_RELIABLE,
    durability: DURABILITY_TRANSIENT_LOCAL,
    deadline: RMW_DURATION_INFINITE,
    lifespan: RMW_DURATION_INFINITE,
    liveliness: POLICY_LIVELINESS_AUTOMATIC,
    liveliness_lease_duration: RMW_DURATION_INFINITE,
    avoid_ros_namespace_conventions: false,
};
