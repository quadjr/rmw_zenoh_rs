#include <fastcdr/Cdr.h>
#include <fastcdr/config.h>
#include <fastcdr/FastBuffer.h>
#include <rosidl_typesupport_fastrtps_c/identifier.h>
#include <rosidl_typesupport_fastrtps_cpp/identifier.hpp>
#include <rosidl_typesupport_fastrtps_cpp/message_type_support.h>
#include <rosidl_typesupport_fastrtps_cpp/service_type_support.h>
#include <rmw/serialized_message.h>
#include "type_support.h"

// Retrieve message type support callbacks for FastRTPS
const void * rs_get_message_type_support_callbacks(
    const rosidl_message_type_support_t *type_support_
){
    const rosidl_message_type_support_t * type_support =
        get_message_typesupport_handle(type_support_, rosidl_typesupport_fastrtps_c__identifier);
    if (!type_support) {
        type_support = get_message_typesupport_handle(
            type_support_, rosidl_typesupport_fastrtps_cpp::typesupport_identifier);
    }
    if (!type_support)  {
        return NULL;
    }
    return type_support->data;
}

// Retrieve service type support callbacks for FastRTPS
const service_type_support_callbacks_t * rs_get_service_type_support_callbacks(
    const rosidl_service_type_support_t *type_support_
){
    const rosidl_service_type_support_t * type_support =
        get_service_typesupport_handle(type_support_, rosidl_typesupport_fastrtps_c__identifier);
    if (!type_support) {
        type_support = get_service_typesupport_handle(
            type_support_, rosidl_typesupport_fastrtps_cpp::typesupport_identifier);
    }
    if (!type_support)  {
        return NULL;
    }
    return static_cast<const service_type_support_callbacks_t *>(type_support->data);
}

// Retrieve request type support callbacks from service type support
const void * rs_get_request_type_support_callbacks(
    const rosidl_service_type_support_t *type_support
){
    const service_type_support_callbacks_t *service_members =
        rs_get_service_type_support_callbacks(type_support);
    if (service_members){
        return service_members->request_members_->data;
    }else{
        return NULL;
    }
}

// Retrieve response type support callbacks from service type support
const void * rs_get_response_type_support_callbacks(
    const rosidl_service_type_support_t *type_support
){
    const service_type_support_callbacks_t *service_members =
        rs_get_service_type_support_callbacks(type_support);
    if (service_members){
        return service_members->response_members_->data;
    }else{
        return NULL;
    }
}

// Retrieve the message name from type support callbacks
const char *rs_get_message_name(
    const void *callbacks_
){
    auto callbacks = static_cast<const message_type_support_callbacks_t *>(callbacks_);
    if(callbacks){
        return callbacks->message_name_;
    }else{
        return NULL;
    }
}

// Retrieve the message namespace from type support callbacks
const char *rs_get_message_namespace(
    const void *callbacks_
){
    auto callbacks = static_cast<const message_type_support_callbacks_t *>(callbacks_);
    if(callbacks){
        return callbacks->message_namespace_;
    }else{
        return NULL;
    }
}

// Calculate the serialized size of a message, including encapsulation
size_t rs_get_serialized_size(
    const message_type_support_callbacks_t * callbacks,
    const void * ros_message
){
  return 4 + callbacks->get_serialized_size(ros_message);
}

// Serialize a ROS message into a serialized message buffer
bool rs_serialize_message(
    const void * callbacks,
    const void * ros_message,
    rmw_serialized_message_t * serialized_message
){
    const message_type_support_callbacks_t * cb =
        static_cast<const message_type_support_callbacks_t *>(callbacks);
    size_t data_length = rs_get_serialized_size(cb, ros_message);
    if (serialized_message->buffer_capacity < data_length) {
        if (rmw_serialized_message_resize(serialized_message, data_length) != RMW_RET_OK) {
            return false;
        }
    }
    serialized_message->buffer_length = data_length;

    eprosima::fastcdr::FastBuffer buffer(
      reinterpret_cast<char *>(serialized_message->buffer), data_length);
    eprosima::fastcdr::Cdr ser(
      buffer, eprosima::fastcdr::Cdr::DEFAULT_ENDIAN, eprosima::fastcdr::Cdr::DDS_CDR);
    ser.serialize_encapsulation();
    return cb->cdr_serialize(ros_message, ser);
}

// Deserialize a serialized message buffer into a ROS message
bool rs_deserialize_message(
    const void * callbacks,
    const rmw_serialized_message_t * serialized_message,
    void * ros_message
){
  const message_type_support_callbacks_t * cb =
    static_cast<const message_type_support_callbacks_t *>(callbacks);
  eprosima::fastcdr::FastBuffer buffer(
    reinterpret_cast<char *>(serialized_message->buffer), serialized_message->buffer_length);
  eprosima::fastcdr::Cdr deser(buffer, eprosima::fastcdr::Cdr::DEFAULT_ENDIAN,
    eprosima::fastcdr::Cdr::DDS_CDR);
  deser.read_encapsulation();
  return cb->cdr_deserialize(deser, ros_message);
}
