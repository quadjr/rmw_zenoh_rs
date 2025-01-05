#ifdef __cplusplus
extern "C" {
#endif

const void * rs_get_message_type_support_callbacks(
    const rosidl_message_type_support_t *type_support_
);
const void * rs_get_request_type_support_callbacks(
    const rosidl_service_type_support_t *type_support
);

const void * rs_get_response_type_support_callbacks(
    const rosidl_service_type_support_t *type_support
);

const char *rs_get_message_name(const void *callbacks);
const char *rs_get_message_namespace(const void *callbacks);

bool rs_serialize_message(
    const void *callbacks,
    const void * ros_message,
    rmw_serialized_message_t * serialized_message
);
bool rs_deserialize_message(
    const void *callbacks,
    const rmw_serialized_message_t * serialized_message,
    void * ros_message
);

#ifdef __cplusplus
}
#endif
