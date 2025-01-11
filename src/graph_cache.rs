use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use zenoh::sample::SampleKind;
use zenoh::Wait;

use crate::Context;
use crate::EndpointInfo;
use crate::EntityType;
use crate::GuardCondition;
use crate::ADMIN_SPACE;

// Represents a graph cache that tracks the state of entities in the system.
pub struct GraphCache {
    #[allow(dead_code)]
    subscriber: zenoh::pubsub::Subscriber<()>,
    endpoint_map: Arc<std::sync::Mutex<BTreeMap<String, EndpointInfo>>>,
    pub guard_condition: Arc<Mutex<GuardCondition>>,
}

impl GraphCache {
    // Constructor for creating a new GraphCache instance
    pub fn new(context: &Context) -> Result<Self, ()> {
        let key_expr = format!("{ADMIN_SPACE}/{0}/**", context.domain_id);
        let endpoint_map = Arc::new(Mutex::new(BTreeMap::new()));
        let endpoint_map_clone = endpoint_map.clone();
        let guard_condition =
            Arc::new(Mutex::new(GuardCondition::new(context.wait_set_cv.clone())));
        let guard_condition_clone = guard_condition.clone();
        Ok(GraphCache {
            subscriber: context
                .session
                .liveliness()
                .declare_subscriber(key_expr)
                .history(true)
                .callback(move |sample| match sample.kind() {
                    SampleKind::Put => {
                        if let Ok(info) = EndpointInfo::try_from(sample.key_expr().as_str()) {
                            if let Ok(mut endpoint_map) = endpoint_map_clone.lock() {
                                endpoint_map.insert(sample.key_expr().to_string(), info);
                                if let Ok(mut guard) = guard_condition_clone.lock() {
                                    guard.trigger();
                                }
                            }
                        }
                    }
                    SampleKind::Delete => {
                        if let Ok(mut endpoint_map) = endpoint_map_clone.lock() {
                            endpoint_map.remove(sample.key_expr().as_str());
                            if let Ok(mut guard) = guard_condition_clone.lock() {
                                guard.trigger();
                            }
                        }
                    }
                })
                .wait()
                .map_err(|_| ())?,
            endpoint_map,
            guard_condition,
        })
    }
    // Retrieves a list of endpoints matching the given filters.
    pub fn get_endpoint_list(
        &self,
        namespace: &str,
        node_name: &str,
        endpoint_name: &str,
        entity_types: &[EntityType],
    ) -> Vec<EndpointInfo> {
        let mut result = Vec::new();
        if let Ok(endpoint_map) = self.endpoint_map.lock() {
            for ep in endpoint_map.values() {
                if ((namespace == "" && node_name == "") || ep.namespace == namespace)
                    && (node_name == "" || ep.node_name == node_name)
                    && (endpoint_name == "" || ep.endpoint_name == endpoint_name)
                    && entity_types.contains(&ep.entity_type)
                {
                    result.push(ep.clone());
                }
            }
        }
        result
    }
    // Counts the number of endpoints matching the given filters.
    pub fn count_endpoint(
        &self,
        namespace: &str,
        node_name: &str,
        endpoint_name: &str,
        entity_types: &[EntityType],
    ) -> usize {
        let mut result = 0;
        if let Ok(endpoint_map) = self.endpoint_map.lock() {
            for ep in endpoint_map.values() {
                if ((namespace == "" && node_name == "") || ep.namespace == namespace)
                    && (node_name == "" || ep.node_name == node_name)
                    && (endpoint_name == "" || ep.endpoint_name == endpoint_name)
                    && entity_types.contains(&ep.entity_type)
                {
                    result += 1;
                }
            }
        }
        result
    }
}
