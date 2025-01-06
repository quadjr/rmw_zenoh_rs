use std::ffi::{c_void, CStr};
use std::ptr::null_mut;

use crate::rmw::*;

// Macro to validate the allocator's function pointers
#[macro_export]
macro_rules! validate_allocator {
    ($return_value:expr, $allocator:expr) => {
        #[allow(unused_unsafe)]
        if unsafe {
            $allocator.allocate.is_none()
                || $allocator.deallocate.is_none()
                || $allocator.reallocate.is_none()
                || $allocator.zero_allocate.is_none()
        } {
            return $return_value;
        }
    };
}

// Macro to check that multiple pointers are not null
#[macro_export]
macro_rules! check_not_null_all {
    ($return_value:expr, $($ptr:expr),+) => {
        $(
            #[allow(unused_unsafe)]
            if unsafe{$ptr.is_null()} {
                return $return_value;
            }
        )+
    };
}

// Macro to check that multiple pointers are null
#[macro_export]
macro_rules! check_is_null_all {
    ($return_value:expr, $($ptr:expr),+) => {
        $(
            #[allow(unused_unsafe)]
            if !unsafe{$ptr.is_null()} {
                return $return_value;
            }
        )+
    };
}

// Macro to validate the implementation identifier of an RMW structure
#[macro_export]
macro_rules! validate_implementation_identifier {
    ($p:expr) => {
        if let Ok(identifier) = unsafe { str_from_ptr((*$p).implementation_identifier) } {
            if identifier != IMPLEMENTATION_IDENTIFIER_STR {
                return RET_INCORRECT_RMW_IMPLEMENTATION;
            }
        } else {
            return RET_INVALID_ARGUMENT;
        }
    };
    ($return_value:expr, $p:expr) => {
        if let Ok(identifier) = unsafe { str_from_ptr((*$p).implementation_identifier) } {
            if identifier != IMPLEMENTATION_IDENTIFIER_STR {
                return $return_value;
            }
        } else {
            return $return_value;
        }
    };
}

// Macro to check that an implementation identifier is empty
#[macro_export]
macro_rules! check_implementation_identifier_empty {
    ($return_value:expr, $p:expr) => {
        if unsafe { !(*$p).implementation_identifier.is_null() } {
            return $return_value;
        }
    };
}

// Implement `Send` and `Sync` for serialized messages and allocator structs
unsafe impl Send for rmw_serialized_message_t {}
unsafe impl Sync for rmw_serialized_message_t {}
unsafe impl Send for rcutils_allocator_t {}
unsafe impl Sync for rcutils_allocator_t {}

// Methods for `rmw_serialized_message_t` for managing serialized message memory
impl rmw_serialized_message_t {
    // Creates a new serialized message with the specified size.
    pub fn new(size: usize, allocator: rcutils_allocator_t) -> Result<Self, ()> {
        let mut res = unsafe { rcutils_get_zero_initialized_uint8_array() };
        (unsafe { rcutils_uint8_array_init(&mut res, size, &allocator) }
            == RMW_RET_OK as rcutils_ret_t)
            .then_some(res)
            .ok_or(())
    }
    // Reserves or resizes memory for the serialized message.
    pub fn try_reserve(&mut self, new_size: usize) -> Result<(), ()> {
        (new_size <= self.buffer_capacity
            || unsafe { rcutils_uint8_array_resize(self, new_size) == RMW_RET_OK as rcutils_ret_t })
        .then_some(())
        .ok_or(())
    }
    // Releases memory for the serialized message.
    pub fn fini(&mut self) {
        unsafe { rcutils_uint8_array_fini(self) };
    }
}

// `StringStorage` struct for managing dynamically allocated strings
pub struct StringStorage<'a> {
    pub string: *mut ::std::os::raw::c_char,
    pub ref_str: &'a str,
    pub allocator: rcutils_allocator_t,
}

impl<'a> StringStorage<'a> {
    // Creates a new `StringStorage` by copying a source string.
    pub fn copy_from(
        src: *const ::std::os::raw::c_char,
        allocator: rcutils_allocator_t,
    ) -> Result<Self, ()> {
        validate_allocator!(Err(()), allocator);

        if src.is_null() {
            return Ok(Self {
                string: null_mut(),
                ref_str: "",
                allocator: allocator.clone(),
            });
        }
        let res = unsafe { rcutils_strdup(src, allocator) };
        if res.is_null() {
            Err(())
        } else {
            let Ok(ref_str) = str_from_ptr(res) else {
                if let Some(deallocate) = allocator.deallocate {
                    unsafe { deallocate(res as *mut c_void, allocator.state) };
                }
                return Err(());
            };
            Ok(Self {
                string: res,
                ref_str,
                allocator: allocator.clone(),
            })
        }
    }
    // Takes ownership of the allocated string, resetting the internal state.
    pub fn take(&mut self) -> *mut ::std::os::raw::c_char {
        let p = self.string;
        self.string = null_mut();
        self.ref_str = "";
        p
    }
}
// Automatically deallocates the string when `StringStorage` is dropped
impl<'a> Drop for StringStorage<'a> {
    fn drop(&mut self) {
        if !self.string.is_null() {
            if let Some(deallocate) = self.allocator.deallocate {
                unsafe { deallocate(self.string as *mut c_void, self.allocator.state) };
            }
        }
    }
}

// Converts a C-style string pointer to a Rust string slice.
pub fn str_from_ptr<'a>(ptr: *const ::std::os::raw::c_char) -> Result<&'a str, ()> {
    if ptr.is_null() {
        Err(())
    } else {
        Ok(unsafe { CStr::from_ptr(ptr).to_str().map_err(|_| ())? })
    }
}
