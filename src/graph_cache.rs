use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use zenoh::sample::SampleKind;
use zenoh::Wait;

use crate::Context;
use crate::EndpointInfo;
use crate::EntityType;
use crate::GuardCondition;
use crate::ADMIN_SPACE;

pub struct GraphCache {
    #[allow(dead_code)]
    subscriber: zenoh::pubsub::Subscriber<()>,
    endpoint_map: Arc<std::sync::Mutex<BTreeMap<String, EndpointInfo>>>,
    pub guard_condition: Arc<GuardCondition>,
}

impl GraphCache {
    pub fn new(context: &Context) -> Result<Self, ()> {
        let key_expr = format!("{ADMIN_SPACE}/{0}/**", context.domain_id);
        let endpoint_map = Arc::new(Mutex::new(BTreeMap::new()));
        let endpoint_map_clone = endpoint_map.clone();
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
                            }
                        }
                    }
                    SampleKind::Delete => {
                        if let Ok(mut endpoint_map) = endpoint_map_clone.lock() {
                            endpoint_map.remove(sample.key_expr().as_str());
                        }
                    }
                })
                .wait()
                .map_err(|_| ())?,
            endpoint_map,
            guard_condition: Arc::new(GuardCondition::new(context.wait_set_cv.clone())),
        })
    }

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
}
