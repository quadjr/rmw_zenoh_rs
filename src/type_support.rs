use std::ffi::CStr;

use crate::rmw::rmw_serialized_message_t;
use crate::rmw::rosidl_message_type_support_t;
use crate::rmw::rosidl_service_type_support_t;
use crate::rmw::rs_deserialize_message;
use crate::rmw::rs_get_message_name;
use crate::rmw::rs_get_message_namespace;
use crate::rmw::rs_get_message_type_support_callbacks;
use crate::rmw::rs_get_request_type_support_callbacks;
use crate::rmw::rs_get_response_type_support_callbacks;
use crate::rmw::rs_serialize_message;

// Represents type support for ROS messages or services, including serialization and deserialization.
pub struct TypeSupport {
    pub type_name: String,
    type_support: *const ::std::os::raw::c_void,
}

// Enable thread-safe usage of `TypeSupport`
unsafe impl Send for TypeSupport {}
unsafe impl Sync for TypeSupport {}

impl TypeSupport {
    // Creates a new `TypeSupport` for a message type.
    pub fn new_message_type_support(
        type_support: *const rosidl_message_type_support_t,
    ) -> Result<Self, ()> {
        let type_support = unsafe { rs_get_message_type_support_callbacks(type_support) };
        Ok(TypeSupport {
            type_name: Self::get_type_name(type_support, "")?,
            type_support,
        })
    }
    // Creates a new `TypeSupport` for a service request type.
    pub fn new_request_type_support(
        type_support: *const rosidl_service_type_support_t,
    ) -> Result<Self, ()> {
        let type_support = unsafe { rs_get_request_type_support_callbacks(type_support) };
        Ok(TypeSupport {
            type_name: Self::get_type_name(type_support, "_Request")?,
            type_support,
        })
    }
    // Creates a new `TypeSupport` for a service response type.
    pub fn new_response_type_support(
        type_support: *const rosidl_service_type_support_t,
    ) -> Result<Self, ()> {
        let type_support = unsafe { rs_get_response_type_support_callbacks(type_support) };
        Ok(TypeSupport {
            type_name: Self::get_type_name(type_support, "_Response")?,
            type_support,
        })
    }
    // Retrieves the fully qualified type name.
    fn get_type_name(
        type_support: *const ::std::os::raw::c_void,
        type_name_suffix: &str,
    ) -> Result<String, ()> {
        // Get namespace and name
        let message_namespace_c = unsafe { rs_get_message_namespace(type_support) };
        let message_name_c = unsafe { rs_get_message_name(type_support) };
        if message_namespace_c.is_null() || message_name_c.is_null() {
            return Err(());
        }
        let message_namespace = unsafe {
            CStr::from_ptr(message_namespace_c)
                .to_str()
                .map_err(|_| ())?
        };
        let mut message_name = unsafe { CStr::from_ptr(message_name_c).to_str().map_err(|_| ())? };

        // Remove suffix
        if message_name.ends_with(type_name_suffix) {
            message_name = &message_name[..message_name.len() - type_name_suffix.len()];
        } else {
            return Err(());
        }
        Ok(format!("{message_namespace}/{message_name}").replace("::", "/"))
    }
    // Serializes a ROS message into a serialized message buffer.
    pub fn serialize(
        &self,
        ros_message: *const ::std::os::raw::c_void,
        serialized_message: *mut rmw_serialized_message_t,
    ) -> Result<(), ()> {
        match unsafe { rs_serialize_message(self.type_support, ros_message, serialized_message) } {
            true => Ok(()),
            false => Err(()),
        }
    }
    // Deserializes a serialized message buffer into a ROS message.
    pub fn deserialize(
        &self,
        serialized_message: *const rmw_serialized_message_t,
        ros_message: *mut ::std::os::raw::c_void,
    ) -> Result<(), ()> {
        match unsafe { rs_deserialize_message(self.type_support, serialized_message, ros_message) }
        {
            true => Ok(()),
            false => Err(()),
        }
    }
}
