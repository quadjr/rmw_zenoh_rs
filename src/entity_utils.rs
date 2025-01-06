use std::ptr::copy_nonoverlapping;
use zenoh::bytes::ZBytes;
use zenoh_ext::ZDeserializer;
use zenoh_ext::ZSerializer;

use crate::rmw::rmw_serialized_message_t;
use crate::RMW_GID_STORAGE_SIZE;
use crate::RMW_GID_STORAGE_SIZE_IRON;

// Reads the payload data from a `ZBytes` into an `rmw_serialized_message_t` buffer.
pub fn read_payload(payload: &ZBytes, msg: &mut rmw_serialized_message_t) -> Result<(), ()> {
    // Reserve space in the message buffer
    msg.try_reserve(payload.len())?;
    msg.buffer_length = payload.len();
    // Copy each slice of the payload into the buffer
    let mut offset: usize = 0;
    for slice in payload.slices() {
        unsafe { copy_nonoverlapping(slice.as_ptr(), msg.buffer.add(offset), slice.len()) };
        offset += slice.len();
    }
    Ok(())
}

/// A trait defining basic behaviors for wait sets.
pub trait WaitSetTrait {
    fn is_empty(&self) -> bool;
    fn cleanup(&mut self) {}
}

// The `Attachment` struct holds metadata for a message, such as sequence number, timestamp, and GID.
pub struct Attachment {
    pub sequence_number: i64,
    pub source_timestamp: i64,
    pub source_gid: [i8; RMW_GID_STORAGE_SIZE_IRON as usize],
}

impl Attachment {
    // Constructor for creating a new Attachment instance
    pub fn new(
        sequence_number: i64,
        source_timestamp: i64,
        source_gid: [u8; RMW_GID_STORAGE_SIZE as usize],
    ) -> Self {
        // Convert the `u8` GID to `i8` format
        let mut source_gid_i8 = [0i8; RMW_GID_STORAGE_SIZE_IRON];
        for i in 0..source_gid_i8.len() {
            source_gid_i8[i] = source_gid[i] as i8;
        }
        Self {
            sequence_number,
            source_timestamp,
            source_gid: source_gid_i8,
        }
    }
}
// Implements conversion from `ZBytes` to `Attachment`.
impl TryFrom<&ZBytes> for Attachment {
    type Error = ();
    fn try_from(value: &ZBytes) -> Result<Self, Self::Error> {
        let mut sequence_number: Option<i64> = None;
        let mut source_timestamp: Option<i64> = None;
        let mut source_gid: Option<[i8; RMW_GID_STORAGE_SIZE_IRON as usize]> = None;
        let mut deserializer = ZDeserializer::new(&value);
        while !deserializer.done() {
            match deserializer.deserialize::<String>() {
                Ok(val) if val == "sequence_number" => {
                    sequence_number = Some(deserializer.deserialize::<i64>().map_err(|_| ())?)
                }
                Ok(val) if val == "source_timestamp" => {
                    source_timestamp = Some(deserializer.deserialize::<i64>().map_err(|_| ())?)
                }
                Ok(val) if val == "source_gid" => {
                    source_gid = Some(
                        deserializer
                            .deserialize::<[i8; RMW_GID_STORAGE_SIZE_IRON as usize]>()
                            .map_err(|_| ())?,
                    );
                }
                _ => return Err(()),
            }
        }
        Ok(Attachment {
            sequence_number: sequence_number.ok_or(())?,
            source_timestamp: source_timestamp.ok_or(())?,
            source_gid: source_gid.ok_or(())?,
        })
    }
}

// Implements conversion from `Attachment` to `ZBytes`.
impl TryFrom<Attachment> for ZBytes {
    type Error = ();
    fn try_from(value: Attachment) -> Result<Self, Self::Error> {
        let mut serializer = ZSerializer::new();
        serializer.serialize("sequence_number");
        serializer.serialize(value.sequence_number);
        serializer.serialize("source_timestamp");
        serializer.serialize(value.source_timestamp);
        serializer.serialize("source_gid");
        serializer.serialize(&value.source_gid);
        Ok(serializer.finish())
    }
}
