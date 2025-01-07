use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;
use std::ptr::null_mut;

use crate::check_not_null_all;
use crate::rmw::*;
use crate::rsutils::str_from_ptr;
use crate::validate_allocator;
use crate::validate_implementation_identifier;
use crate::EndpointInfo;
use crate::EntityType;
use crate::Node;
use crate::IMPLEMENTATION_IDENTIFIER_STR;

// Retrieves endpoint information for a given topic.
pub fn get_endpoint_info_by_topic(
    node: *const rmw_node_t,
    allocator: *mut rcutils_allocator_t,
    endpoint_name: *const ::std::os::raw::c_char,
    _no_mangle: bool, // Not used
    endpoint_types: &[EntityType],
    info_array: *mut rmw_topic_endpoint_info_array_t,
) -> rmw_ret_t {
    // Validate inputs and the is empty
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        node,
        (*node).data,
        allocator,
        info_array
    );
    validate_allocator!(RET_INVALID_ARGUMENT, (*allocator));
    validate_implementation_identifier!(node);
    if unsafe { rmw_topic_endpoint_info_array_check_zero(info_array) } != RET_OK {
        return RET_INVALID_ARGUMENT;
    }
    // Retrieve endpoint information from the graph cache
    let Ok(endpoint_name) = str_from_ptr(endpoint_name) else {
        return RET_INVALID_ARGUMENT;
    };
    let graph_cache = unsafe { &(*((*node).data as *mut Node)).graph_cache };
    let info = graph_cache.get_endpoint_list("", "", endpoint_name, endpoint_types);
    if info.is_empty() {
        return RET_OK;
    };
    // Initialize and populate `info_array` with endpoint information
    if unsafe {
        rmw_topic_endpoint_info_array_init_with_size(&mut *info_array, info.len(), allocator)
    } == RET_OK
        && info.iter().enumerate().all(|(index, item)| {
            let info_s = unsafe { (*info_array).info_array.add(index) };
            set_endpoint_info(item, info_s, allocator).is_ok()
        })
    {
        RET_OK
    } else {
        unsafe { rmw_topic_endpoint_info_array_fini(&mut *info_array, allocator) };
        RET_ERROR
    }
}

/// Sets detailed endpoint information into rmw_topic_endpoint_info_t.
pub fn set_endpoint_info(
    item: &EndpointInfo,
    info: *mut rmw_topic_endpoint_info_t,
    allocator: *mut rcutils_allocator_t,
) -> Result<(), ()> {
    let gid = item.get_gid();
    let endpoint_type = match item.entity_type {
        EntityType::Publisher => rmw_endpoint_type_e_RMW_ENDPOINT_PUBLISHER,
        EntityType::Subscriber => rmw_endpoint_type_e_RMW_ENDPOINT_SUBSCRIPTION,
        _ => rmw_endpoint_type_e_RMW_ENDPOINT_INVALID,
    };
    let (node_name, namespace, topic_type) = (
        CString::new(item.node_name.clone()).map_err(|_| ())?,
        CString::new(item.namespace.clone()).map_err(|_| ())?,
        CString::new(item.endpoint_type.clone()).map_err(|_| ())?,
    );
    [
        unsafe { rmw_topic_endpoint_info_set_node_name(info, node_name.as_ptr(), allocator) },
        unsafe { rmw_topic_endpoint_info_set_node_namespace(info, namespace.as_ptr(), allocator) },
        unsafe { rmw_topic_endpoint_info_set_topic_type(info, topic_type.as_ptr(), allocator) },
        unsafe { rmw_topic_endpoint_info_set_endpoint_type(info, endpoint_type) },
        unsafe { rmw_topic_endpoint_info_set_gid(info, gid.as_ptr(), gid.len()) },
        unsafe { rmw_topic_endpoint_info_set_qos_profile(info, &item.qos) },
    ]
    .iter()
    .all(|&result| result == RET_OK)
    .then_some(())
    .ok_or(())
}
// Retrieves topic names and types for a specific node.
pub fn get_names_and_types(
    node: *const rmw_node_t,
    allocator: *mut rcutils_allocator_t,
    _no_demangle: bool,
    node_name: *const ::std::os::raw::c_char,
    namespace: *const ::std::os::raw::c_char,
    endpoint_types: &[EntityType],
    names_and_types: *mut rmw_names_and_types_t,
) -> rmw_ret_t {
    // Validate inputs and ensure arrays are empty
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        node,
        (*node).data,
        allocator,
        names_and_types
    );
    validate_allocator!(RET_INVALID_ARGUMENT, (*allocator));
    validate_implementation_identifier!(node);
    if unsafe { rmw_names_and_types_check_zero(names_and_types) } != RET_OK {
        return RET_INVALID_ARGUMENT;
    };
    // Validate node name and namespace if provided
    if !node_name.is_null() && !namespace.is_null() {
        let mut name_valid = NODE_NAME_VALID;
        let mut ns_valid = NAMESPACE_VALID;
        if unsafe { rmw_validate_node_name(node_name, &mut name_valid, null_mut()) } != RET_OK
            || unsafe { rmw_validate_namespace(namespace, &mut ns_valid, null_mut()) } != RET_OK
            || NODE_NAME_VALID != name_valid
            || NAMESPACE_VALID != ns_valid
        {
            return RET_INVALID_ARGUMENT;
        }
    }
    // Check the node exists
    let node_name = str_from_ptr(node_name).unwrap_or("");
    let namespace = str_from_ptr(namespace).unwrap_or("");
    let graph_cache = unsafe { &(*((*node).data as *mut Node)).graph_cache };
    let node_info = graph_cache.get_endpoint_list(namespace, node_name, "", &[EntityType::Node]);
    if node_info.is_empty() {
        return RET_NODE_NAME_NON_EXISTENT;
    }
    // Retrieve endpoint information from the graph cache
    let info = graph_cache.get_endpoint_list(namespace, node_name, "", endpoint_types);
    if info.is_empty() {
        return RET_OK;
    }
    // Generate map of topic names and types
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for item in &info {
        map.entry(item.endpoint_name.clone())
            .or_insert_with(BTreeSet::new)
            .insert(item.endpoint_type.clone());
    }
    // Initialize and populate `names_and_types` with topic information
    if unsafe { rmw_names_and_types_init(&mut *names_and_types, map.len(), allocator) } != RET_OK {
        return RET_ERROR;
    }
    /// Sets topic names and types into rmw_names_and_types_t.
    if map.iter().enumerate().all(|(name_index, item)| unsafe {
        if let Ok(topic_name) = CString::new(item.0.clone()) {
            std::ptr::write(
                (*names_and_types).names.data.add(name_index),
                rcutils_strdup(topic_name.as_ptr(), *allocator),
            );
            rcutils_string_array_init(
                (*names_and_types).types.add(name_index),
                item.1.len(),
                allocator,
            );
            item.1.iter().enumerate().all(|(type_index, sub_item)| {
                if let Ok(endpoint_type) = CString::new(sub_item.clone()) {
                    std::ptr::write(
                        (*((*names_and_types).types.add(name_index)))
                            .data
                            .add(type_index),
                        rcutils_strdup(endpoint_type.as_ptr(), *allocator),
                    );

                    true
                } else {
                    false
                }
            })
        } else {
            false
        }
    }) {
        RET_OK
    } else {
        // Finalize rmw_names_and_types_t if failed.
        unsafe { rcutils_string_array_fini(&mut (*names_and_types).names) };
        unsafe {
            for i in 0..(*names_and_types).names.size {
                rcutils_string_array_fini((*names_and_types).types.add(i));
            }
        }
        RET_ERROR
    }
}

// Retrieves node names, namespaces, and enclaves for all nodes in the graph cache.
pub fn get_node_names(
    node: *const rmw_node_t,
    node_names: *mut rcutils_string_array_t,
    node_namespaces: *mut rcutils_string_array_t,
    enclaves: *mut rcutils_string_array_t,
) -> rmw_ret_t {
    // Validate inputs and ensure arrays are empty
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        node,
        (*node).data,
        node_names,
        node_namespaces
    );
    validate_implementation_identifier!(node);
    if unsafe { rmw_check_zero_rmw_string_array(node_names) } != RET_OK
        || unsafe { rmw_check_zero_rmw_string_array(node_namespaces) } != RET_OK
        || (!enclaves.is_null() && unsafe { rmw_check_zero_rmw_string_array(enclaves) } != RET_OK)
    {
        return RET_INVALID_ARGUMENT;
    }
    // Retrieve node information from the graph cache
    let node = unsafe { &*((*node).data as *mut Node) };
    let info = node
        .graph_cache
        .get_endpoint_list("", "", "", &[EntityType::Node]);
    if info.is_empty() {
        return RET_OK;
    }
    // Initialize the output structs
    let allocator = &node.context.allocator;
    if unsafe { rcutils_string_array_init(node_names, info.len(), allocator) } != RET_OK
        || unsafe { rcutils_string_array_init(node_namespaces, info.len(), allocator) } != RET_OK
        || (!enclaves.is_null()
            && unsafe { rcutils_string_array_init(enclaves, info.len(), allocator) } != RET_OK)
    {
        return RET_ERROR;
    }
    // Populate node names, namespaces, and enclaves
    for (i, ep) in info.iter().enumerate() {
        let node_name = CString::new(ep.node_name.clone()).unwrap();
        let namespace = CString::new(ep.namespace.clone()).unwrap();
        let enclave = CString::new(ep.enclave.clone()).unwrap();
        unsafe {
            std::ptr::write(
                (*node_names).data.add(i),
                rcutils_strdup(node_name.as_ptr(), (*node_names).allocator),
            );
            std::ptr::write(
                (*node_namespaces).data.add(i),
                rcutils_strdup(namespace.as_ptr(), (*node_namespaces).allocator),
            );
            if !enclaves.is_null() {
                std::ptr::write(
                    (*enclaves).data.add(i),
                    rcutils_strdup(enclave.as_ptr(), (*enclaves).allocator),
                );
            }
        }
    }
    RET_OK
}
