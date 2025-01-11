#ifdef __cplusplus
extern "C"
{
#endif

    // Retrieve message type support callbacks for FastRTPS
    const void *rs_get_message_type_support_callbacks(
        const rosidl_message_type_support_t *type_support_);
    // Retrieve request type support callbacks from service type support
    const void *rs_get_request_type_support_callbacks(
        const rosidl_service_type_support_t *type_support);
    // Retrieve response type support callbacks from service type support
    const void *rs_get_response_type_support_callbacks(
        const rosidl_service_type_support_t *type_support);
    // Retrieve the message name from type support callbacks
    const char *rs_get_message_name(const void *callbacks);
    // Retrieve the message namespace from type support callbacks
    const char *rs_get_message_namespace(const void *callbacks);
    // Serialize a ROS message into a serialized message buffer
    bool rs_serialize_message(
        const void *callbacks,
        const void *ros_message,
        rmw_serialized_message_t *serialized_message);
    // Deserialize a serialized message buffer into a ROS message
    bool rs_deserialize_message(
        const void *callbacks,
        const rmw_serialized_message_t *serialized_message,
        void *ros_message);

#ifdef __cplusplus
}
#endif
