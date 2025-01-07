use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use zenoh::Wait;

use crate::rmw::rmw_guard_condition_t;
use crate::Context;
use crate::EndpointInfo;
use crate::EntityType;
use crate::GraphCache;

// Node struct: Represents a ROS 2 node
pub struct Node<'a> {
    pub context: &'a Context,
    pub info: EndpointInfo,
    pub graph_cache: Arc<GraphCache>,
    pub graph_guard_condition: Option<Box<rmw_guard_condition_t>>,
    next_entity_id: AtomicUsize,
    #[allow(dead_code)]
    liveliness_token: zenoh::liveliness::LivelinessToken,
}

impl<'a> Node<'a> {
    // Constructor for creating a new Node instance
    pub fn new(context: &'a mut Context, namespace: &str, node_name: &str) -> Result<Self, ()> {
        // Initialize EndpointInfo with node-specific metadata
        let mut info = EndpointInfo::default();
        info.domain_id = context.domain_id;
        info.z_id = context.session.info().zid().wait().to_string();
        info.node_id = context.generate_node_id();
        info.entity_id = 0;
        info.entity_type = EntityType::Node;
        info.enclave = context.enclave.clone();
        info.namespace = namespace.to_string();
        info.node_name = node_name.to_string();

        // Convert EndpointInfo into a key expression for liveliness
        let key_expr = info.to_string();
        Ok(Node {
            context,
            info,
            graph_cache: Arc::new(GraphCache::new(context)?),
            graph_guard_condition: None,
            next_entity_id: AtomicUsize::new(1), // 0 is reserved for Node
            liveliness_token: context
                .session
                .liveliness()
                .declare_token(key_expr)
                .wait()
                .map_err(|_| ())?,
        })
    }
    // Generates a unique entity ID by incrementing the counter atomically
    pub fn generate_entity_id(&mut self) -> usize {
        return self.next_entity_id.fetch_add(1, Ordering::Relaxed);
    }
}
