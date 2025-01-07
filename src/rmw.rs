#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(improper_ctypes)]
#![allow(non_snake_case)]

use std::ptr::addr_of;
use std::ptr::{null, null_mut};
use std::time::Duration;

// Include bidgen header
#[allow(dead_code)]
pub mod rmw {
    include!("bindings.rs");
}
pub use rmw::*;

// Macros
use crate::check_implementation_identifier_empty;
use crate::check_is_null_all;
use crate::check_not_null_all;
use crate::validate_allocator;
use crate::validate_implementation_identifier;

// Some staff
use crate::graph_cache_utils::get_endpoint_info_by_topic;
use crate::graph_cache_utils::get_names_and_types;
use crate::graph_cache_utils::get_node_names;
use crate::rsutils::str_from_ptr;
use crate::Client;
use crate::Context;
use crate::EntityType;
use crate::Event;
use crate::GuardCondition;
use crate::Node;
use crate::Publisher;
use crate::Service;
use crate::StringStorage;
use crate::Subscriber;
use crate::TypeSupport;
use crate::WaitSetTrait;
use crate::IMPLEMENTATION_IDENTIFIER_CHAR;
use crate::IMPLEMENTATION_IDENTIFIER_STR;
use crate::SERIALIZATION_FORMAT_CHAR;
use rmw_localhost_only_e_RMW_LOCALHOST_ONLY_ENABLED as LOCALHOST_ONLY_ENABLED;

// Varible type conversion
pub const RET_OK: rmw_ret_t = RMW_RET_OK as rmw_ret_t;
pub const RET_ERROR: rmw_ret_t = RMW_RET_ERROR as rmw_ret_t;
pub const RET_TIMEOUT: rmw_ret_t = RMW_RET_TIMEOUT as rmw_ret_t;
pub const RET_UNSUPPORTED: rmw_ret_t = RMW_RET_UNSUPPORTED as rmw_ret_t;
pub const RET_BAD_ALLOC: rmw_ret_t = RMW_RET_BAD_ALLOC as rmw_ret_t;
pub const RET_INVALID_ARGUMENT: rmw_ret_t = RMW_RET_INVALID_ARGUMENT as rmw_ret_t;
pub const RET_INCORRECT_RMW_IMPLEMENTATION: rmw_ret_t =
    RMW_RET_INCORRECT_RMW_IMPLEMENTATION as rmw_ret_t;
pub const RET_NODE_NAME_NON_EXISTENT: rmw_ret_t = RMW_RET_NODE_NAME_NON_EXISTENT as rmw_ret_t;
pub const NODE_NAME_VALID: rmw_ret_t = RMW_NODE_NAME_VALID as rmw_ret_t;
pub const NAMESPACE_VALID: rmw_ret_t = RMW_NAMESPACE_VALID as rmw_ret_t;
pub const TOPIC_VALID: rmw_ret_t = RMW_TOPIC_VALID as rmw_ret_t;

#[no_mangle]
pub extern "C" fn rmw_get_implementation_identifier() -> *const ::std::os::raw::c_char {
    IMPLEMENTATION_IDENTIFIER_CHAR
}

#[no_mangle]
pub extern "C" fn rmw_get_serialization_format() -> *const ::std::os::raw::c_char {
    SERIALIZATION_FORMAT_CHAR
}

#[no_mangle]
pub extern "C" fn rmw_init_options_init(
    init_options: *mut rmw_init_options_t,
    allocator: rcutils_allocator_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, init_options);
    check_implementation_identifier_empty!(RET_INVALID_ARGUMENT, init_options);
    validate_allocator!(RET_INVALID_ARGUMENT, allocator);

    unsafe {
        (*init_options).instance_id = 0;
        (*init_options).implementation_identifier = rmw_get_implementation_identifier();
        (*init_options).domain_id = RMW_DEFAULT_DOMAIN_ID as usize;
        (*init_options).security_options = rmw_get_zero_initialized_security_options();
        (*init_options).localhost_only = rmw_localhost_only_e_RMW_LOCALHOST_ONLY_DEFAULT;
        (*init_options).enclave = null_mut();
        (*init_options).allocator = allocator;
        (*init_options).impl_ = null_mut();
    }
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_init_options_copy(
    src: *const rmw_init_options_t,
    dst: *mut rmw_init_options_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, src, dst);
    validate_allocator!(RET_INVALID_ARGUMENT, (*src).allocator);
    validate_implementation_identifier!(src);
    check_implementation_identifier_empty!(RET_INVALID_ARGUMENT, dst);

    let src = unsafe { &*src };
    let mut tmp = src.clone();
    let Ok(mut enclave) = StringStorage::copy_from(src.enclave, src.allocator) else {
        return RET_ERROR;
    };
    if unsafe {
        rmw_security_options_copy(
            &src.security_options,
            &src.allocator,
            &mut tmp.security_options,
        )
    } != RET_OK
    {
        return RET_ERROR;
    }

    tmp.implementation_identifier = rmw_get_implementation_identifier();
    tmp.enclave = enclave.take();
    unsafe { *dst = tmp };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_init_options_fini(init_options: *mut rmw_init_options_t) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, init_options);
    validate_allocator!(RET_INVALID_ARGUMENT, (*init_options).allocator);
    validate_implementation_identifier!(init_options);

    let opt = unsafe { &mut *init_options };
    if unsafe { rmw_security_options_fini(&mut opt.security_options, &opt.allocator) } != RET_OK {
        return RET_ERROR;
    }
    unsafe { *opt = rmw_get_zero_initialized_init_options() };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_init(
    options: *const rmw_init_options_t,
    context: *mut rmw_context_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, options, (*options).enclave, context);
    validate_implementation_identifier!(options);
    check_implementation_identifier_empty!(RET_INVALID_ARGUMENT, context);

    let options = unsafe { &*options };
    let context = unsafe { &mut *context };
    let Ok(enclave) = str_from_ptr(options.enclave) else {
        return RET_ERROR;
    };
    // Determine the domain ID; default to 0 if unspecified
    let domain_id = if options.domain_id == RMW_DEFAULT_DOMAIN_ID as usize {
        0
    } else {
        options.domain_id
    };

    if let Ok(ctx) = Context::new(
        domain_id,
        options.localhost_only == LOCALHOST_ONLY_ENABLED,
        enclave,
        options.allocator,
    ) {
        if rmw_init_options_copy(options, &mut context.options) != RET_OK {
            return RET_ERROR;
        }
        context.instance_id = options.instance_id;
        context.implementation_identifier = rmw_get_implementation_identifier();
        context.actual_domain_id = domain_id;
        context.impl_ = Box::into_raw(Box::new(ctx)) as *mut rmw_context_impl_t;

        RET_OK
    } else {
        RET_ERROR
    }
}

#[no_mangle]
pub extern "C" fn rmw_shutdown(context: *mut rmw_context_t) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, context);
    validate_implementation_identifier!(context);

    if unsafe { !(*context).impl_.is_null() } {
        let _ = unsafe { Box::from_raw((*context).impl_) };
        unsafe { (*context).impl_ = null_mut() };
    }
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_context_fini(context: *mut rmw_context_t) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, context);
    validate_implementation_identifier!(context);
    check_is_null_all!(RET_INVALID_ARGUMENT, (*context).impl_);

    if unsafe { rmw_init_options_fini(&mut (*context).options) } != RET_OK {
        return RET_ERROR;
    }
    unsafe { *context = rmw_get_zero_initialized_context() };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_create_guard_condition(
    context: *mut rmw_context_t,
) -> *mut rmw_guard_condition_t {
    check_not_null_all!(null_mut(), context, (*context).impl_);
    validate_implementation_identifier!(null_mut(), context);

    let ctx_impl = unsafe { &mut *((*context).impl_ as *mut Context) };
    let guard_condition = GuardCondition::new(ctx_impl.wait_set_cv.clone());

    Box::into_raw(Box::new(rmw_guard_condition_t {
        implementation_identifier: rmw_get_implementation_identifier(),
        data: Box::into_raw(Box::new(guard_condition)) as *mut ::std::os::raw::c_void,
        context: context,
    }))
}

#[no_mangle]
pub extern "C" fn rmw_destroy_guard_condition(
    guard_condition: *mut rmw_guard_condition_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        guard_condition,
        (*guard_condition).data
    );
    validate_implementation_identifier!(guard_condition);

    let guard_condition = unsafe { Box::from_raw(guard_condition) };
    let _ = unsafe { Box::from_raw(guard_condition.data as *mut GuardCondition) };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_create_node(
    context: *mut rmw_context_t,
    name: *const ::std::os::raw::c_char,
    namespace: *const ::std::os::raw::c_char,
) -> *mut rmw_node_t {
    check_not_null_all!(null_mut(), context, (*context).impl_, name, namespace);
    validate_implementation_identifier!(null_mut(), context);

    let mut name_valid = NODE_NAME_VALID;
    let mut namespace_valid = NAMESPACE_VALID;
    if unsafe { rmw_validate_node_name(name, &mut name_valid, null_mut()) } != RET_OK
        || unsafe { rmw_validate_namespace(namespace, &mut namespace_valid, null_mut()) } != RET_OK
        || NODE_NAME_VALID != name_valid
        || NAMESPACE_VALID != namespace_valid
    {
        return null_mut();
    }

    let ctx = unsafe { &mut *context };
    let (Ok(mut name), Ok(mut namespace)) = (
        StringStorage::copy_from(name, ctx.options.allocator),
        StringStorage::copy_from(namespace, ctx.options.allocator),
    ) else {
        return null_mut();
    };

    let ctx_impl = unsafe { &mut *(ctx.impl_ as *mut Context) };
    let Ok(mut node) = Node::new(ctx_impl, namespace.ref_str, name.ref_str) else {
        return null_mut();
    };

    node.graph_guard_condition = Some(Box::new(rmw_guard_condition_t {
        implementation_identifier: rmw_get_implementation_identifier(),
        data: addr_of!(*node.graph_cache.guard_condition) as *mut ::std::os::raw::c_void,
        context: context,
    }));

    Box::into_raw(Box::new(rmw_node_t {
        implementation_identifier: rmw_get_implementation_identifier(),
        data: Box::into_raw(Box::new(node)) as *mut ::std::os::raw::c_void,
        name: name.take(),
        namespace_: namespace.take(),
        context: context,
    }))
}

#[no_mangle]
pub extern "C" fn rmw_destroy_node(node: *mut rmw_node_t) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, node, (*node).data);
    validate_implementation_identifier!(node);

    let node_impl = unsafe { &mut *((*node).data as *mut Node) };
    let allocator = &node_impl.context.allocator;
    if let Some(deallocate) = allocator.deallocate {
        unsafe { deallocate((*node).name as *mut std::ffi::c_void, allocator.state) };
        unsafe { deallocate((*node).namespace_ as *mut std::ffi::c_void, allocator.state) };
    } else {
        return RET_ERROR;
    }

    let _ = unsafe { *Box::from_raw((*node).data as *mut Node) };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_init_publisher_allocation(
    _type_support: *const rosidl_message_type_support_t,
    _message_bounds: *const rosidl_runtime_c__Sequence__bound,
    _allocation: *mut rmw_publisher_allocation_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Not used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_fini_publisher_allocation(
    _allocation: *mut rmw_publisher_allocation_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Not used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_create_publisher(
    node: *const rmw_node_t,
    type_support: *const rosidl_message_type_support_t,
    topic_name: *const ::std::os::raw::c_char,
    qos_profile: *const rmw_qos_profile_t,
    publisher_options: *const rmw_publisher_options_t,
) -> *mut rmw_publisher_t {
    check_not_null_all!(
        null_mut(),
        node,
        (*node).data,
        type_support,
        topic_name,
        qos_profile,
        publisher_options
    );
    validate_implementation_identifier!(null_mut(), node);

    let node_impl = unsafe { &mut *((*node).data as *mut Node) };

    let qos = unsafe { &*qos_profile };

    if !qos.is_valid() {
        return null_mut();
    }

    let mut topic_valid: i32 = 0;
    if unsafe { rmw_validate_full_topic_name(topic_name, &mut topic_valid, null_mut()) } != RET_OK
        || (topic_valid != TOPIC_VALID && !qos.avoid_ros_namespace_conventions)
    {
        return null_mut();
    }

    let Ok(mut topic_name) = StringStorage::copy_from(topic_name, node_impl.context.allocator)
    else {
        return null_mut();
    };

    let Ok(type_support) = TypeSupport::new_message_type_support(type_support) else {
        return null_mut();
    };

    let Ok(publisher) = Publisher::new(node_impl, topic_name.ref_str, type_support, *qos) else {
        return null_mut();
    };

    Box::into_raw(Box::new(rmw_publisher_t {
        implementation_identifier: rmw_get_implementation_identifier(),
        data: Box::into_raw(Box::new(publisher)) as *mut ::std::os::raw::c_void,
        topic_name: topic_name.take(),
        options: unsafe { *publisher_options },
        can_loan_messages: false,
    }))
}

#[no_mangle]
pub extern "C" fn rmw_destroy_publisher(
    node: *mut rmw_node_t,
    publisher: *mut rmw_publisher_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        node,
        (*node).data,
        publisher,
        (*publisher).data
    );
    validate_implementation_identifier!(node);
    validate_implementation_identifier!(publisher);

    let node_impl = unsafe { &mut *((*node).data as *mut Node) };
    let allocator = &node_impl.context.allocator;
    if let Some(deallocate) = allocator.deallocate {
        unsafe {
            deallocate(
                (*publisher).topic_name as *mut std::ffi::c_void,
                allocator.state,
            )
        };
    } else {
        return RET_ERROR;
    }

    let publisher = unsafe { Box::from_raw(publisher) };
    let _ = unsafe { Box::from_raw(publisher.data as *mut Publisher) };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_borrow_loaned_message(
    _publisher: *const rmw_publisher_t,
    _type_support: *const rosidl_message_type_support_t,
    _ros_message: *mut *mut ::std::os::raw::c_void,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_return_loaned_message_from_publisher(
    _publisher: *const rmw_publisher_t,
    _loaned_message: *mut ::std::os::raw::c_void,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_publish_serialized_message(
    publisher: *const rmw_publisher_t,
    serialized_message: *const rmw_serialized_message_t,
    _allocation: *mut rmw_publisher_allocation_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        publisher,
        (*publisher).data,
        serialized_message,
        (*serialized_message).buffer
    );
    validate_implementation_identifier!(publisher);
    let serialized_message = unsafe { &*serialized_message };
    if serialized_message.buffer_length <= 0 {
        return RET_INVALID_ARGUMENT;
    }

    let pub_impl = unsafe { &mut *((*publisher).data as *mut Publisher) };
    match pub_impl.publish_serialized_message(serialized_message) {
        Ok(_) => RET_OK,
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_publish(
    publisher: *const rmw_publisher_t,
    ros_message: *const ::std::os::raw::c_void,
    _allocation: *mut rmw_publisher_allocation_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        publisher,
        (*publisher).data,
        ros_message
    );
    validate_implementation_identifier!(publisher);

    let pub_impl = unsafe { &mut *((*publisher).data as *mut Publisher) };
    match pub_impl.publish(ros_message) {
        Ok(_) => RET_OK,
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_publish_loaned_message(
    _publisher: *const rmw_publisher_t,
    _ros_message: *mut ::std::os::raw::c_void,
    _allocation: *mut rmw_publisher_allocation_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_publisher_count_matched_subscriptions(
    publisher: *const rmw_publisher_t,
    subscription_count: *mut usize,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        publisher,
        (*publisher).data,
        subscription_count
    );
    validate_implementation_identifier!(publisher);

    let pub_impl = unsafe { &mut *((*publisher).data as *mut Publisher) };
    let count = pub_impl.endpoint.graph_cache.count_endpoint(
        "",
        "",
        &pub_impl.endpoint.info.endpoint_name,
        &[EntityType::Subscriber],
    );
    unsafe { *subscription_count = count };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_publisher_get_actual_qos(
    publisher: *const rmw_publisher_t,
    qos: *mut rmw_qos_profile_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, publisher, (*publisher).data, qos);
    validate_implementation_identifier!(publisher);

    let pub_impl = unsafe { &mut *((*publisher).data as *mut Publisher) };
    unsafe { (*qos) = pub_impl.endpoint.info.qos };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_get_serialized_message_size(
    _type_support: *const rosidl_message_type_support_t,
    _message_bounds: *const rosidl_runtime_c__Sequence__bound,
    _size: *mut usize,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Not used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_publisher_assert_liveliness(publisher: *const rmw_publisher_t) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, publisher, (*publisher).data);
    validate_implementation_identifier!(publisher);
    RET_OK // Publisher is always alive
}

#[no_mangle]
pub extern "C" fn rmw_publisher_wait_for_all_acked(
    publisher: *const rmw_publisher_t,
    _wait_timeout: rmw_time_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, publisher, (*publisher).data);
    validate_implementation_identifier!(publisher);
    RET_OK // Always OK because zenoh doesn't have ack mechanism
}

#[no_mangle]
pub extern "C" fn rmw_serialize(
    ros_message: *const ::std::os::raw::c_void,
    type_support: *const rosidl_message_type_support_t,
    serialized_message: *mut rmw_serialized_message_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        ros_message,
        type_support,
        serialized_message
    );
    let Ok(type_support) = TypeSupport::new_message_type_support(type_support) else {
        return RET_ERROR;
    };
    match type_support.serialize(ros_message, serialized_message) {
        Ok(_) => RET_OK,
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_deserialize(
    serialized_message: *const rmw_serialized_message_t,
    type_support: *const rosidl_message_type_support_t,
    ros_message: *mut ::std::os::raw::c_void,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        serialized_message,
        type_support,
        ros_message
    );
    let Ok(type_support) = TypeSupport::new_message_type_support(type_support) else {
        return RET_ERROR;
    };
    match type_support.deserialize(serialized_message, ros_message) {
        Ok(_) => RET_OK,
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_init_subscription_allocation(
    _type_support: *const rosidl_message_type_support_t,
    _message_bounds: *const rosidl_runtime_c__Sequence__bound,
    _allocation: *mut rmw_subscription_allocation_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Not used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_fini_subscription_allocation(
    _allocation: *mut rmw_subscription_allocation_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Not used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_create_subscription(
    node: *const rmw_node_t,
    type_support: *const rosidl_message_type_support_t,
    topic_name: *const ::std::os::raw::c_char,
    qos_policies: *const rmw_qos_profile_t,
    subscription_options: *const rmw_subscription_options_t,
) -> *mut rmw_subscription_t {
    check_not_null_all!(
        null_mut(),
        node,
        (*node).data,
        type_support,
        topic_name,
        qos_policies,
        subscription_options
    );
    validate_implementation_identifier!(null_mut(), node);
    let node_impl = unsafe { &mut *((*node).data as *mut Node) };
    let qos = unsafe { &*qos_policies };
    if !qos.is_valid() {
        return null_mut();
    }

    // Validate topic name
    let mut topic_valid: i32 = 0;
    if unsafe { rmw_validate_full_topic_name(topic_name, &mut topic_valid, null_mut()) } != RET_OK
        || (topic_valid != TOPIC_VALID && !qos.avoid_ros_namespace_conventions)
    {
        return null_mut();
    }
    let Ok(mut topic_name) = StringStorage::copy_from(topic_name, node_impl.context.allocator)
    else {
        return null_mut();
    };

    let Ok(type_support) = TypeSupport::new_message_type_support(type_support) else {
        return null_mut();
    };

    let Ok(subscriber) = Subscriber::new(node_impl, topic_name.ref_str, type_support, *qos) else {
        return null_mut();
    };

    Box::into_raw(Box::new(rmw_subscription_t {
        implementation_identifier: rmw_get_implementation_identifier(),
        data: Box::into_raw(Box::new(subscriber)) as *mut ::std::os::raw::c_void,
        topic_name: topic_name.take(),
        options: unsafe { *subscription_options },
        can_loan_messages: false,
        is_cft_enabled: false,
    }))
}

#[no_mangle]
pub extern "C" fn rmw_destroy_subscription(
    node: *mut rmw_node_t,
    subscription: *mut rmw_subscription_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        node,
        (*node).data,
        subscription,
        (*subscription).data
    );
    validate_implementation_identifier!(node);
    validate_implementation_identifier!(subscription);

    let node_impl = unsafe { &mut *((*node).data as *mut Node) };
    let allocator = &node_impl.context.allocator;
    if let Some(deallocate) = allocator.deallocate {
        unsafe {
            deallocate(
                (*subscription).topic_name as *mut std::ffi::c_void,
                allocator.state,
            )
        };
    } else {
        return RET_ERROR;
    }

    let subscriber = unsafe { Box::from_raw(subscription) };
    let _ = unsafe { Box::from_raw(subscriber.data as *mut Subscriber) };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_subscription_count_matched_publishers(
    subscription: *const rmw_subscription_t,
    publisher_count: *mut usize,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        subscription,
        (*subscription).data,
        publisher_count
    );
    validate_implementation_identifier!(subscription);

    let sub_impl = unsafe { &mut *((*subscription).data as *mut Subscriber) };
    let count = sub_impl.endpoint.graph_cache.count_endpoint(
        "",
        "",
        &sub_impl.endpoint.info.endpoint_name,
        &[EntityType::Publisher],
    );
    unsafe { *publisher_count = count };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_subscription_get_actual_qos(
    subscription: *const rmw_subscription_t,
    qos: *mut rmw_qos_profile_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        subscription,
        (*subscription).data,
        qos
    );
    validate_implementation_identifier!(subscription);

    let sub_impl = unsafe { &mut *((*subscription).data as *mut Subscriber) };
    unsafe { (*qos) = sub_impl.endpoint.info.qos };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_subscription_set_content_filter(
    _subscription: *mut rmw_subscription_t,
    _options: *const rmw_subscription_content_filter_options_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_subscription_get_content_filter(
    _subscription: *const rmw_subscription_t,
    _allocator: *mut rcutils_allocator_t,
    _options: *mut rmw_subscription_content_filter_options_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_take_serialized_message_with_info(
    subscription: *const rmw_subscription_t,
    serialized_message: *mut rmw_serialized_message_t,
    taken: *mut bool,
    message_info: *mut rmw_message_info_t,
    _allocation: *mut rmw_subscription_allocation_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        subscription,
        (*subscription).data,
        serialized_message,
        taken,
        message_info
    );
    validate_implementation_identifier!(subscription);

    let subscriber = unsafe { &mut *((*subscription).data as *mut Subscriber) };
    let serialized_message = unsafe { &mut *serialized_message };
    match subscriber.take_serialized_message(serialized_message, message_info) {
        Ok(res_taken) => {
            unsafe { *taken = res_taken };
            RET_OK
        }
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_take_serialized_message(
    subscription: *const rmw_subscription_t,
    serialized_message: *mut rmw_serialized_message_t,
    taken: *mut bool,
    _allocation: *mut rmw_subscription_allocation_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        subscription,
        (*subscription).data,
        serialized_message,
        taken
    );
    validate_implementation_identifier!(subscription);

    let subscriber = unsafe { &mut *((*subscription).data as *mut Subscriber) };
    let serialized_message = unsafe { &mut *serialized_message };
    match subscriber.take_serialized_message(serialized_message, null_mut()) {
        Ok(res_taken) => {
            unsafe { *taken = res_taken };
            RET_OK
        }
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_take_with_info(
    subscription: *const rmw_subscription_t,
    ros_message: *mut ::std::os::raw::c_void,
    taken: *mut bool,
    message_info: *mut rmw_message_info_t,
    _allocation: *mut rmw_subscription_allocation_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        subscription,
        (*subscription).data,
        ros_message,
        taken,
        message_info
    );
    validate_implementation_identifier!(subscription);

    let sub_impl = unsafe { &mut *((*subscription).data as *mut Subscriber) };
    match sub_impl.take_message(ros_message, message_info) {
        Ok(res_taken) => {
            unsafe { *taken = res_taken };
            RET_OK
        }
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_take(
    subscription: *const rmw_subscription_t,
    ros_message: *mut ::std::os::raw::c_void,
    taken: *mut bool,
    _allocation: *mut rmw_subscription_allocation_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        subscription,
        (*subscription).data,
        ros_message,
        taken
    );
    validate_implementation_identifier!(subscription);

    let sub_impl = unsafe { &mut *((*subscription).data as *mut Subscriber) };
    match sub_impl.take_message(ros_message, null_mut()) {
        Ok(res_taken) => {
            unsafe { *taken = res_taken };
            RET_OK
        }
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_take_sequence(
    _subscription: *const rmw_subscription_t,
    _count: usize,
    _message_sequence: *mut rmw_message_sequence_t,
    _message_info_sequence: *mut rmw_message_info_sequence_t,
    _taken: *mut usize,
    _allocation: *mut rmw_subscription_allocation_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_take_loaned_message(
    _subscription: *const rmw_subscription_t,
    _loaned_message: *mut *mut ::std::os::raw::c_void,
    _taken: *mut bool,
    _allocation: *mut rmw_subscription_allocation_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Not used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_take_loaned_message_with_info(
    _subscription: *const rmw_subscription_t,
    _loaned_message: *mut *mut ::std::os::raw::c_void,
    _taken: *mut bool,
    _message_info: *mut rmw_message_info_t,
    _allocation: *mut rmw_subscription_allocation_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_return_loaned_message_from_subscription(
    _subscription: *const rmw_subscription_t,
    _loaned_message: *mut ::std::os::raw::c_void,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_create_client(
    node: *const rmw_node_t,
    type_support: *const rosidl_service_type_support_t,
    service_name: *const ::std::os::raw::c_char,
    qos_policies: *const rmw_qos_profile_t,
) -> *mut rmw_client_t {
    check_not_null_all!(
        null_mut(),
        node,
        (*node).data,
        type_support,
        service_name,
        qos_policies
    );
    validate_implementation_identifier!(null_mut(), node);
    let node_impl = unsafe { &mut *((*node).data as *mut Node) };
    let qos = unsafe { &*qos_policies };
    if !qos.is_valid() {
        return null_mut();
    }

    // Validate service name
    let mut service_valid: i32 = 0;
    if unsafe { rmw_validate_full_topic_name(service_name, &mut service_valid, null_mut()) }
        != RET_OK
        || (service_valid != TOPIC_VALID && !qos.avoid_ros_namespace_conventions)
    {
        return null_mut();
    }
    let Ok(mut service_name) = StringStorage::copy_from(service_name, node_impl.context.allocator)
    else {
        return null_mut();
    };

    let (Ok(request_type_support), Ok(response_type_support)) = (
        TypeSupport::new_request_type_support(type_support),
        TypeSupport::new_response_type_support(type_support),
    ) else {
        return null_mut();
    };

    let Ok(client) = Client::new(
        node_impl,
        service_name.ref_str,
        request_type_support,
        response_type_support,
        *qos,
    ) else {
        return null_mut();
    };

    Box::into_raw(Box::new(rmw_client_t {
        implementation_identifier: rmw_get_implementation_identifier(),
        data: Box::into_raw(Box::new(client)) as *mut ::std::os::raw::c_void,
        service_name: service_name.take(),
    }))
}

#[no_mangle]
pub extern "C" fn rmw_destroy_client(
    node: *mut rmw_node_t,
    client: *mut rmw_client_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        node,
        (*node).data,
        client,
        (*client).data
    );
    validate_implementation_identifier!(node);
    validate_implementation_identifier!(client);

    let node_impl = unsafe { &mut *((*node).data as *mut Node) };
    let allocator = &node_impl.context.allocator;
    if let Some(deallocate) = allocator.deallocate {
        unsafe {
            deallocate(
                (*client).service_name as *mut std::ffi::c_void,
                allocator.state,
            )
        };
    } else {
        return RET_ERROR;
    }

    let client = unsafe { Box::from_raw(client) };
    let _ = unsafe { Box::from_raw(client.data as *mut Client) };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_send_request(
    client: *const rmw_client_t,
    ros_request: *const ::std::os::raw::c_void,
    sequence_id: *mut i64,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        client,
        (*client).data,
        ros_request,
        sequence_id
    );
    validate_implementation_identifier!(client);

    let client_impl = unsafe { &mut *((*client).data as *mut Client) };
    match client_impl.send_request(ros_request) {
        Ok(seq) => {
            unsafe { *sequence_id = seq };
            RET_OK
        }
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_take_response(
    client: *const rmw_client_t,
    request_header: *mut rmw_service_info_t,
    ros_response: *mut ::std::os::raw::c_void,
    taken: *mut bool,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        client,
        (*client).data,
        request_header,
        ros_response,
        taken
    );
    validate_implementation_identifier!(client);
    let client_impl = unsafe { &mut *((*client).data as *mut Client) };
    match client_impl.take_response(request_header, ros_response) {
        Ok(res_taken) => {
            unsafe { *taken = res_taken };
            RET_OK
        }
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_client_request_publisher_get_actual_qos(
    client: *const rmw_client_t,
    qos: *mut rmw_qos_profile_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, client, (*client).data, qos);
    validate_implementation_identifier!(client);

    let client_impl = unsafe { &mut *((*client).data as *mut Client) };
    unsafe { (*qos) = client_impl.endpoint.info.qos };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_client_response_subscription_get_actual_qos(
    client: *const rmw_client_t,
    qos: *mut rmw_qos_profile_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, client, (*client).data, qos);
    validate_implementation_identifier!(client);

    let client_impl = unsafe { &mut *((*client).data as *mut Client) };
    unsafe { (*qos) = client_impl.endpoint.info.qos };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_service_server_is_available(
    node: *const rmw_node_t,
    client: *const rmw_client_t,
    is_available: *mut bool,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        node,
        (*node).data,
        client,
        (*client).data,
        is_available
    );
    validate_implementation_identifier!(node);
    validate_implementation_identifier!(client);

    let node = unsafe { &*((*node).data as *mut Node) };
    let client = unsafe { &*((*client).data as *mut Client) };
    let endpoint_name = &(client.endpoint.info.endpoint_name);
    let count = node
        .graph_cache
        .count_endpoint("", "", endpoint_name, &[EntityType::Service]);
    unsafe { *is_available = count > 0 };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_create_service(
    node: *const rmw_node_t,
    type_support: *const rosidl_service_type_support_t,
    service_name: *const ::std::os::raw::c_char,
    qos_profile: *const rmw_qos_profile_t,
) -> *mut rmw_service_t {
    check_not_null_all!(
        null_mut(),
        node,
        (*node).data,
        type_support,
        service_name,
        qos_profile
    );
    validate_implementation_identifier!(null_mut(), node);
    let node_impl = unsafe { &mut *((*node).data as *mut Node) };
    let qos = unsafe { &*qos_profile };
    if !qos.is_valid() {
        return null_mut();
    }

    // Validate service name
    let mut service_valid: i32 = 0;
    if unsafe { rmw_validate_full_topic_name(service_name, &mut service_valid, null_mut()) }
        != RET_OK
        || (service_valid != TOPIC_VALID && !qos.avoid_ros_namespace_conventions)
    {
        return null_mut();
    }
    let Ok(mut service_name) = StringStorage::copy_from(service_name, node_impl.context.allocator)
    else {
        return null_mut();
    };

    let (Ok(request_type_support), Ok(response_type_support)) = (
        TypeSupport::new_request_type_support(type_support),
        TypeSupport::new_response_type_support(type_support),
    ) else {
        return null_mut();
    };

    let Ok(service) = Service::new(
        node_impl,
        service_name.ref_str,
        request_type_support,
        response_type_support,
        *qos,
    ) else {
        return null_mut();
    };

    Box::into_raw(Box::new(rmw_service_t {
        implementation_identifier: rmw_get_implementation_identifier(),
        data: Box::into_raw(Box::new(service)) as *mut ::std::os::raw::c_void,
        service_name: service_name.take(),
    }))
}

#[no_mangle]
pub extern "C" fn rmw_destroy_service(
    node: *mut rmw_node_t,
    service: *mut rmw_service_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        node,
        (*node).data,
        service,
        (*service).data
    );
    validate_implementation_identifier!(node);
    validate_implementation_identifier!(service);

    let node_impl = unsafe { &mut *((*node).data as *mut Node) };
    let allocator = &node_impl.context.allocator;
    if let Some(deallocate) = allocator.deallocate {
        unsafe {
            deallocate(
                (*service).service_name as *mut std::ffi::c_void,
                allocator.state,
            )
        };
    } else {
        return RET_ERROR;
    }

    let service = unsafe { Box::from_raw(service) };
    let _ = unsafe { Box::from_raw(service.data as *mut Service) };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_take_request(
    service: *const rmw_service_t,
    request_header: *mut rmw_service_info_t,
    ros_request: *mut ::std::os::raw::c_void,
    taken: *mut bool,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        service,
        (*service).data,
        request_header,
        ros_request,
        taken
    );
    validate_implementation_identifier!(service);

    let service_impl = unsafe { &mut *((*service).data as *mut Service) };
    match service_impl.take_request(request_header, ros_request) {
        Ok(res_taken) => {
            unsafe { *taken = res_taken };
            return RET_OK;
        }
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_send_response(
    service: *const rmw_service_t,
    request_header: *mut rmw_request_id_t,
    ros_response: *mut ::std::os::raw::c_void,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, service, (*service).data, ros_response);
    validate_implementation_identifier!(service);

    let service_impl = unsafe { &mut *((*service).data as *mut Service) };
    match service_impl.send_response(request_header, ros_response) {
        Ok(_) => RET_OK,
        Err(_) => RET_ERROR,
    }
}

#[no_mangle]
pub extern "C" fn rmw_service_request_subscription_get_actual_qos(
    service: *const rmw_service_t,
    qos: *mut rmw_qos_profile_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, service, (*service).data, qos);
    validate_implementation_identifier!(service);

    let service_impl = unsafe { &mut *((*service).data as *mut Service) };
    unsafe { (*qos) = service_impl.endpoint.info.qos };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_service_response_publisher_get_actual_qos(
    service: *const rmw_service_t,
    qos: *mut rmw_qos_profile_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, service, (*service).data, qos);
    validate_implementation_identifier!(service);

    let service_impl = unsafe { &mut *((*service).data as *mut Service) };
    unsafe { (*qos) = service_impl.endpoint.info.qos };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_node_get_graph_guard_condition(
    node: *const rmw_node_t,
) -> *const rmw_guard_condition_t {
    check_not_null_all!(null(), node, (*node).data);
    validate_implementation_identifier!(null(), node);

    let node_impl = unsafe { &mut *((*node).data as *mut Node) };
    match &node_impl.graph_guard_condition {
        Some(guard_condition) => addr_of!(**guard_condition),
        None => null(),
    }
}

#[no_mangle]
pub extern "C" fn rmw_trigger_guard_condition(
    guard_condition: *const rmw_guard_condition_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        guard_condition,
        (*guard_condition).data
    );
    validate_implementation_identifier!(guard_condition);
    let guard_condition = unsafe { &mut *((*guard_condition).data as *mut GuardCondition) };
    guard_condition.trigger();
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_create_wait_set(
    context: *mut rmw_context_t,
    _max_conditions: usize,
) -> *mut rmw_wait_set_t {
    check_not_null_all!(null_mut(), context, (*context).impl_);
    validate_implementation_identifier!(null_mut(), context);
    Box::into_raw(Box::new(rmw_wait_set_t {
        implementation_identifier: rmw_get_implementation_identifier(),
        guard_conditions: null_mut(),
        data: rmw_create_guard_condition(context) as *mut ::std::os::raw::c_void,
    }))
}

#[no_mangle]
pub extern "C" fn rmw_destroy_wait_set(wait_set: *mut rmw_wait_set_t) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, wait_set, (*wait_set).data);
    validate_implementation_identifier!(wait_set);
    let wait_set = unsafe { Box::from_raw(wait_set) };
    rmw_destroy_guard_condition(wait_set.data as *mut rmw_guard_condition_t);
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_wait(
    subscriptions: *mut rmw_subscriptions_t,
    guard_conditions: *mut rmw_guard_conditions_t,
    services: *mut rmw_services_t,
    clients: *mut rmw_clients_t,
    events: *mut rmw_events_t,
    wait_set: *mut rmw_wait_set_t,
    wait_timeout: *const rmw_time_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, wait_set, (*wait_set).data);
    validate_implementation_identifier!(wait_set);

    unsafe {
        let mut items_ptr: Vec<*mut *mut ::std::os::raw::c_void> = Vec::new();
        let mut items: Vec<&mut dyn WaitSetTrait> = Vec::new();

        // Macro to collect WaitSetTrait items and their pointers.
        macro_rules! collect_functions {
            ($target:ident, $objects:ident, $count:ident, $type:ty) => {
                if !$target.is_null() && !(*$target).$objects.is_null() {
                    for i in 0..((*$target).$count) {
                        let item = (*$target).$objects.add(i);
                        items_ptr.push(item);
                        items.push(&mut *(*item as *mut $type));
                    }
                }
            };
        }
        // Collect subscribers, guard conditions, services, and clients into `items`.
        collect_functions!(subscriptions, subscribers, subscriber_count, Subscriber);
        collect_functions!(
            guard_conditions,
            guard_conditions,
            guard_condition_count,
            GuardCondition
        );
        collect_functions!(services, services, service_count, Service);
        collect_functions!(clients, clients, client_count, Client);

        // Handle events, which have a different data structure.
        if !events.is_null() && !(*events).events.is_null() {
            for i in 0..((*events).event_count) {
                let item = (*events).events.add(i);
                items_ptr.push(item);
                items.push(&mut *((*(*item as *const rmw_event_t)).data as *mut Event));
            }
        }
        // If no items are collected, return a timeout immediately.
        if items.len() == 0 {
            return RET_TIMEOUT;
        }

        // Prepare the condition variable associated with the guard condition in the wait set.
        let mut data_ready = false;
        let guard_condition =
            (*((*wait_set).data as *mut rmw_guard_condition_t)).data as *mut GuardCondition;
        let (lock, cvar) = &*(*guard_condition).wait_set_cv;
        if let Ok(lock) = lock.lock() {
            // Wait for data to become available.
            if wait_timeout.is_null() {
                // Wait indefinitely if wait_timeout is null.
                let _unused = cvar.wait_while(lock, |_| items.iter().all(|item| item.is_empty()));
            } else if (*wait_timeout).sec != 0 || (*wait_timeout).nsec != 0 {
                // Wait for the specified duration.
                let duration = Duration::new((*wait_timeout).sec, (*wait_timeout).nsec as u32);
                let _unused = cvar.wait_timeout_while(lock, duration, |_| {
                    items.iter().all(|item| item.is_empty())
                });
            } else {
                // No wait if wait_timeout is 0.
            }
            // Process the items after the wait.
            for i in 0..items.len() {
                if items[i].is_empty() {
                    // Mark empty items as null.
                    *items_ptr[i] = std::ptr::null_mut();
                } else {
                    // Cleanup the item and mark data as ready.
                    items[i].cleanup();
                    data_ready = true;
                }
            }
        } else {
            return RET_ERROR;
        }

        match data_ready {
            true => RET_OK,
            false => RET_TIMEOUT,
        }
    }
}

#[no_mangle]
pub extern "C" fn rmw_subscription_set_on_new_message_callback(
    subscription: *mut rmw_subscription_t,
    callback: rmw_event_callback_t,
    user_data: *const ::std::os::raw::c_void,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, subscription, (*subscription).data);
    validate_implementation_identifier!(subscription);

    let subscriber = unsafe { &mut *((*subscription).data as *mut Subscriber) };
    if let Ok(mut on_recv_callback) = subscriber.endpoint.on_recv_callback.lock() {
        *on_recv_callback = (callback, user_data as usize);
        RET_OK
    } else {
        RET_ERROR
    }
}

#[no_mangle]
pub extern "C" fn rmw_service_set_on_new_request_callback(
    service: *mut rmw_service_t,
    callback: rmw_event_callback_t,
    user_data: *const ::std::os::raw::c_void,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, service, (*service).data);
    validate_implementation_identifier!(service);

    let service = unsafe { &mut *((*service).data as *mut Service) };
    if let Ok(mut on_recv_callback) = service.endpoint.on_recv_callback.lock() {
        *on_recv_callback = (callback, user_data as usize);
        RET_OK
    } else {
        RET_ERROR
    }
}

#[no_mangle]
pub extern "C" fn rmw_client_set_on_new_response_callback(
    client: *mut rmw_client_t,
    callback: rmw_event_callback_t,
    user_data: *const ::std::os::raw::c_void,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, client, (*client).data);
    validate_implementation_identifier!(client);

    let client = unsafe { &mut *((*client).data as *mut Client) };
    if let Ok(mut on_recv_callback) = client.endpoint.on_recv_callback.lock() {
        *on_recv_callback = (callback, user_data as usize);
        RET_OK
    } else {
        RET_ERROR
    }
}

#[no_mangle]
pub extern "C" fn rmw_event_set_callback(
    event: *mut rmw_event_t,
    callback: rmw_event_callback_t,
    user_data: *const ::std::os::raw::c_void,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, event, (*event).data);
    validate_implementation_identifier!(event);

    let event = unsafe { &mut *((*event).data as *mut Event) };
    event.event_callback = Some((callback, user_data as usize));
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_qos_profile_check_compatible(
    _publisher_profile: rmw_qos_profile_t,
    _subscription_profile: rmw_qos_profile_t,
    compatibility: *mut rmw_qos_compatibility_type_t,
    reason: *mut ::std::os::raw::c_char,
    reason_size: usize,
) -> rmw_ret_t {
    unsafe {
        if reason_size != 0 {
            *reason = b'\0' as std::os::raw::c_char;
        }
        *compatibility = rmw_qos_compatibility_type_e_RMW_QOS_COMPATIBILITY_OK;
    }
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_publisher_event_init(
    rmw_event: *mut rmw_event_t,
    publisher: *const rmw_publisher_t,
    event_type: rmw_event_type_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        rmw_event,
        publisher,
        (*publisher).data
    );
    check_implementation_identifier_empty!(RET_INVALID_ARGUMENT, rmw_event);
    validate_implementation_identifier!(publisher);

    let pub_impl = unsafe { &mut *((*publisher).data as *mut Publisher) };
    if let Ok(mut events) = pub_impl.endpoint.events.lock() {
        if events.contains_key(&event_type) {
            return RET_ERROR;
        }
        let mut event = Box::new(Event::new(event_type));
        unsafe {
            (*rmw_event).implementation_identifier = rmw_get_implementation_identifier();
            (*rmw_event).data =
                Box::as_mut(&mut event) as *mut Event as *mut ::std::os::raw::c_void;
            (*rmw_event).event_type = event_type;
        }
        events.insert(event_type, event);
        RET_OK
    } else {
        RET_ERROR
    }
}

#[no_mangle]
pub extern "C" fn rmw_subscription_event_init(
    rmw_event: *mut rmw_event_t,
    subscription: *const rmw_subscription_t,
    event_type: rmw_event_type_t,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        rmw_event,
        subscription,
        (*subscription).data
    );
    check_implementation_identifier_empty!(RET_INVALID_ARGUMENT, rmw_event);
    validate_implementation_identifier!(subscription);

    let sub_impl = unsafe { &mut *((*subscription).data as *mut Subscriber) };
    if let Ok(mut events) = sub_impl.endpoint.events.lock() {
        if events.contains_key(&event_type) {
            return RET_ERROR;
        }
        let mut event = Box::new(Event::new(event_type));
        unsafe {
            (*rmw_event).implementation_identifier = rmw_get_implementation_identifier();
            (*rmw_event).data =
                Box::as_mut(&mut event) as *mut Event as *mut ::std::os::raw::c_void;
            (*rmw_event).event_type = event_type;
        }
        events.insert(event_type, event);
        RET_OK
    } else {
        RET_ERROR
    }
}

#[no_mangle]
pub extern "C" fn rmw_take_event(
    event_handle: *const rmw_event_t,
    event_info: *mut ::std::os::raw::c_void,
    taken: *mut bool,
) -> rmw_ret_t {
    check_not_null_all!(
        RET_INVALID_ARGUMENT,
        event_handle,
        (*event_handle).data,
        event_info,
        taken
    );
    validate_implementation_identifier!(event_handle);

    unsafe { *taken = false };
    return RET_OK;

    // let event_handle = unsafe { &*event_handle };
    // // let event_map = unsafe { &mut *(event_handle.data as *mut EventMap) };
    // match event_handle.event_type {
    //     rmw_event_type_e_RMW_EVENT_REQUESTED_QOS_INCOMPATIBLE => unsafe {
    //         let event_info = event_info as *mut rmw_requested_qos_incompatible_event_status_t;
    //         (*event_info).total_count = 0;
    //         (*event_info).total_count_change = 0;
    //         (*event_info).last_policy_kind = rmw_qos_policy_kind_e_RMW_QOS_POLICY_INVALID;
    //     },
    //     rmw_event_type_e_RMW_EVENT_OFFERED_QOS_INCOMPATIBLE => unsafe {
    //         let event_info = event_info as *mut rmw_offered_qos_incompatible_event_status_t;
    //         (*event_info).total_count = 0;
    //         (*event_info).total_count_change = 0;
    //         (*event_info).last_policy_kind = rmw_qos_policy_kind_e_RMW_QOS_POLICY_INVALID;
    //     },
    //     rmw_event_type_e_RMW_EVENT_MESSAGE_LOST => unsafe {
    //         let event_info = event_info as *mut rmw_message_lost_status_t;
    //         (*event_info).total_count = 0;
    //         (*event_info).total_count_change = 0;
    //     },
    //     _ => {
    //         return RET_ERROR;
    //     }
    // }
    // unsafe { *taken = true };
    // RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_get_node_names(
    node: *const rmw_node_t,
    node_names: *mut rcutils_string_array_t,
    node_namespaces: *mut rcutils_string_array_t,
) -> rmw_ret_t {
    get_node_names(node, node_names, node_namespaces, std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn rmw_get_node_names_with_enclaves(
    node: *const rmw_node_t,
    node_names: *mut rcutils_string_array_t,
    node_namespaces: *mut rcutils_string_array_t,
    enclaves: *mut rcutils_string_array_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, enclaves);
    get_node_names(node, node_names, node_namespaces, enclaves)
}

#[no_mangle]
pub extern "C" fn rmw_count_publishers(
    node: *const rmw_node_t,
    topic_name: *const ::std::os::raw::c_char,
    count: *mut usize,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, node, (*node).data, topic_name, count);
    validate_implementation_identifier!(node);

    let mut topic_valid: i32 = 0;
    if unsafe { rmw_validate_full_topic_name(topic_name, &mut topic_valid, null_mut()) } != RET_OK
        || topic_valid != TOPIC_VALID
    {
        return RET_INVALID_ARGUMENT;
    }

    let Ok(topic_name) = str_from_ptr(topic_name) else {
        return RET_INVALID_ARGUMENT;
    };

    let node = unsafe { &*((*node).data as *mut Node) };
    let endpoint_count =
        node.graph_cache
            .count_endpoint("", "", topic_name, &[EntityType::Publisher]);
    unsafe {
        *count = endpoint_count;
    }
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_count_subscribers(
    node: *const rmw_node_t,
    topic_name: *const ::std::os::raw::c_char,
    count: *mut usize,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, node, (*node).data, topic_name, count);
    validate_implementation_identifier!(node);

    let mut topic_valid: i32 = 0;
    if unsafe { rmw_validate_full_topic_name(topic_name, &mut topic_valid, null_mut()) } != RET_OK
        || topic_valid != TOPIC_VALID
    {
        return RET_INVALID_ARGUMENT;
    }

    let Ok(topic_name) = str_from_ptr(topic_name) else {
        return RET_INVALID_ARGUMENT;
    };

    let node = unsafe { &*((*node).data as *mut Node) };
    let endpoint_count =
        node.graph_cache
            .count_endpoint("", "", topic_name, &[EntityType::Subscriber]);
    unsafe {
        *count = endpoint_count;
    }
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_get_publishers_info_by_topic(
    node: *const rmw_node_t,
    allocator: *mut rcutils_allocator_t,
    topic_name: *const ::std::os::raw::c_char,
    no_mangle: bool,
    publishers_info: *mut rmw_topic_endpoint_info_array_t,
) -> rmw_ret_t {
    get_endpoint_info_by_topic(
        node,
        allocator,
        topic_name,
        no_mangle,
        &[EntityType::Publisher],
        publishers_info,
    )
}

#[no_mangle]
pub extern "C" fn rmw_get_subscriptions_info_by_topic(
    node: *const rmw_node_t,
    allocator: *mut rcutils_allocator_t,
    topic_name: *const ::std::os::raw::c_char,
    no_mangle: bool,
    subscriptions_info: *mut rmw_topic_endpoint_info_array_t,
) -> rmw_ret_t {
    get_endpoint_info_by_topic(
        node,
        allocator,
        topic_name,
        no_mangle,
        &[EntityType::Subscriber],
        subscriptions_info,
    )
}

#[no_mangle]
pub extern "C" fn rmw_get_topic_names_and_types(
    node: *const rmw_node_t,
    allocator: *mut rcutils_allocator_t,
    no_demangle: bool,
    topic_names_and_types: *mut rmw_names_and_types_t,
) -> rmw_ret_t {
    get_names_and_types(
        node,
        allocator,
        no_demangle,
        null(),
        null(),
        &[EntityType::Subscriber, EntityType::Publisher],
        topic_names_and_types,
    )
}

#[no_mangle]
pub extern "C" fn rmw_publisher_get_network_flow_endpoints(
    _publisher: *const rmw_publisher_t,
    _allocator: *mut rcutils_allocator_t,
    _network_flow_endpoint_array: *mut rmw_network_flow_endpoint_array_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_subscription_get_network_flow_endpoints(
    _subscription: *const rmw_subscription_t,
    _allocator: *mut rcutils_allocator_t,
    _network_flow_endpoint_array: *mut rmw_network_flow_endpoint_array_t,
) -> rmw_ret_t {
    RET_UNSUPPORTED // Used in rcl
}

#[no_mangle]
pub extern "C" fn rmw_get_service_names_and_types(
    node: *const rmw_node_t,
    allocator: *mut rcutils_allocator_t,
    service_names_and_types: *mut rmw_names_and_types_t,
) -> rmw_ret_t {
    get_names_and_types(
        node,
        allocator,
        false,
        null(),
        null(),
        &[EntityType::Service, EntityType::Client],
        service_names_and_types,
    )
}

#[no_mangle]
pub extern "C" fn rmw_get_subscriber_names_and_types_by_node(
    node: *const rmw_node_t,
    allocator: *mut rcutils_allocator_t,
    node_name: *const ::std::os::raw::c_char,
    node_namespace: *const ::std::os::raw::c_char,
    no_demangle: bool,
    topic_names_and_types: *mut rmw_names_and_types_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, node_name, node_namespace);
    get_names_and_types(
        node,
        allocator,
        no_demangle,
        node_name,
        node_namespace,
        &[EntityType::Subscriber],
        topic_names_and_types,
    )
}

#[no_mangle]
pub extern "C" fn rmw_get_publisher_names_and_types_by_node(
    node: *const rmw_node_t,
    allocator: *mut rcutils_allocator_t,
    node_name: *const ::std::os::raw::c_char,
    node_namespace: *const ::std::os::raw::c_char,
    no_demangle: bool,
    topic_names_and_types: *mut rmw_names_and_types_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, node_name, node_namespace);
    get_names_and_types(
        node,
        allocator,
        no_demangle,
        node_name,
        node_namespace,
        &[EntityType::Publisher],
        topic_names_and_types,
    )
}

#[no_mangle]
pub extern "C" fn rmw_get_service_names_and_types_by_node(
    node: *const rmw_node_t,
    allocator: *mut rcutils_allocator_t,
    node_name: *const ::std::os::raw::c_char,
    node_namespace: *const ::std::os::raw::c_char,
    service_names_and_types: *mut rmw_names_and_types_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, node_name, node_namespace);
    get_names_and_types(
        node,
        allocator,
        false,
        node_name,
        node_namespace,
        &[EntityType::Service],
        service_names_and_types,
    )
}

#[no_mangle]
pub extern "C" fn rmw_get_client_names_and_types_by_node(
    node: *const rmw_node_t,
    allocator: *mut rcutils_allocator_t,
    node_name: *const ::std::os::raw::c_char,
    node_namespace: *const ::std::os::raw::c_char,
    service_names_and_types: *mut rmw_names_and_types_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, node_name, node_namespace);
    get_names_and_types(
        node,
        allocator,
        false,
        node_name,
        node_namespace,
        &[EntityType::Client],
        service_names_and_types,
    )
}

#[no_mangle]
pub extern "C" fn rmw_get_gid_for_publisher(
    publisher: *const rmw_publisher_t,
    gid: *mut rmw_gid_t,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, publisher, (*publisher).data, gid);
    validate_implementation_identifier!(publisher);

    let publisher = unsafe { &mut *((*publisher).data as *mut Publisher) };
    unsafe {
        (*gid).implementation_identifier = rmw_get_implementation_identifier();
        (*gid).data = publisher.endpoint.info.get_gid();
    }
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_compare_gids_equal(
    gid1: *const rmw_gid_t,
    gid2: *const rmw_gid_t,
    result: *mut bool,
) -> rmw_ret_t {
    check_not_null_all!(RET_INVALID_ARGUMENT, gid1, gid2);
    validate_implementation_identifier!(gid1);
    validate_implementation_identifier!(gid2);

    let data1 = unsafe { (*gid1).data };
    let data2 = unsafe { (*gid2).data };
    let mut is_same = true;
    for i in 0..data1.len() {
        if data1[i] != data2[i] {
            is_same = false;
            break;
        }
    }
    unsafe { *result = is_same };
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_set_log_severity(_severity: rmw_log_severity_t) -> rmw_ret_t {
    RET_OK
}

#[no_mangle]
pub extern "C" fn rmw_feature_supported(_feature: rmw_feature_t) -> bool {
    false // Not used in rcl
}
