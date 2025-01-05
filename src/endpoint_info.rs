use sha2::{Digest, Sha256};
use std::fmt;
use std::str::FromStr;
use strum::{Display, EnumString};

use crate::rmw::rmw_qos_profile_t;
use crate::ADMIN_SPACE;
use crate::DEFAULT_QOS;
use crate::RMW_GID_STORAGE_SIZE;

#[derive(PartialEq, Clone, EnumString, Display)]
pub enum EntityType {
    #[strum(serialize = "NN")]
    Node,
    #[strum(serialize = "MP")]
    Publisher,
    #[strum(serialize = "MS")]
    Subscriber,
    #[strum(serialize = "SS")]
    Service,
    #[strum(serialize = "SC")]
    Client,
}
impl Default for EntityType {
    fn default() -> EntityType {
        EntityType::Node
    }
}

#[derive(Clone)]
pub struct EndpointInfo {
    pub domain_id: usize,
    pub z_id: String,
    pub node_id: usize,
    pub entity_id: usize,
    pub entity_type: EntityType,
    pub enclave: String,
    pub namespace: String,
    pub node_name: String,
    pub endpoint_name: String,
    pub endpoint_type: String,
    pub endpoint_typehash: String,
    pub qos: rmw_qos_profile_t,
}

impl EndpointInfo {
    pub fn get_endpoint_keyexpr(&self) -> String {
        [
            self.domain_id.to_string(),
            Self::mangle_name(&self.endpoint_name),
            Self::mangle_name(&self.endpoint_type.to_string()),
            Self::mangle_name(&self.endpoint_typehash),
        ]
        .join("/")
    }

    pub fn mangle_name(name: &str) -> String {
        if name == "" {
            return "%".to_string();
        }
        name.replace("/", "%")
    }

    fn demangle_name(name: &str) -> String {
        name.replace("%", "/")
    }

    pub fn get_gid(&self) -> [u8; RMW_GID_STORAGE_SIZE as usize] {
        let mut result = [0; RMW_GID_STORAGE_SIZE as usize];
        let hash = Sha256::digest(self.to_string().as_bytes());
        result.copy_from_slice(&hash[..RMW_GID_STORAGE_SIZE as usize]);
        result
    }
}
impl Default for EndpointInfo {
    fn default() -> Self {
        Self {
            domain_id: 0,
            z_id: "".to_string(),
            node_id: 0,
            entity_id: 0,
            entity_type: EntityType::default(),
            enclave: "".to_string(),
            namespace: "".to_string(),
            node_name: "".to_string(),
            endpoint_name: "".to_string(),
            endpoint_type: "".to_string(),
            endpoint_typehash: "".to_string(),
            qos: DEFAULT_QOS,
        }
    }
}

// Generate EndpointInfo from keyexpr string
impl fmt::Display for EndpointInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut key_expr = [
            ADMIN_SPACE,
            &self.domain_id.to_string(),
            &self.z_id,
            &self.node_id.to_string(),
            &self.entity_id.to_string(),
            &self.entity_type.to_string(),
            &Self::mangle_name(&self.enclave),
            &Self::mangle_name(&self.namespace),
            &Self::mangle_name(&self.node_name),
        ]
        .join("/");
        if self.entity_type != EntityType::Node {
            key_expr += &[
                "",
                &Self::mangle_name(&self.endpoint_name),
                &Self::mangle_name(&self.endpoint_type.to_string()),
                &Self::mangle_name(&self.endpoint_typehash),
                &self.qos.to_string(),
            ]
            .join("/");
        }
        write!(f, "{}", key_expr)?;
        Ok(())
    }
}

// Generate keyexpr string from EmtptyInfo
impl TryFrom<&str> for EndpointInfo {
    type Error = ();
    fn try_from(key_expr: &str) -> Result<Self, Self::Error> {
        let values: Vec<&str> = key_expr.split('/').collect();
        if values.len() < 9 {
            return Err(());
        }
        let is_node = values[5] == EntityType::Node.to_string();
        if values[0] != ADMIN_SPACE || (values.len() < 13 && !is_node) {
            return Err(());
        };

        let mut info = EndpointInfo::default();
        info.domain_id = values[1].parse::<usize>().map_err(|_| ())?;
        info.z_id = values[2].to_string();
        info.node_id = values[3].parse::<usize>().map_err(|_| ())?;
        info.entity_id = values[4].parse::<usize>().map_err(|_| ())?;
        info.entity_type = values[5].parse().map_err(|_| ())?;
        info.enclave = Self::demangle_name(values[6]);
        info.namespace = Self::demangle_name(values[7]);
        info.node_name = Self::demangle_name(values[8]);
        if !is_node {
            info.endpoint_name = Self::demangle_name(values[9]);
            info.endpoint_type = Self::demangle_name(values[10]).parse().map_err(|_| ())?;
            info.endpoint_typehash = Self::demangle_name(values[11]);
            info.qos = Self::demangle_name(values[12]).parse().map_err(|_| ())?;
        }
        Ok(info)
    }
}

// Generate keyexpr string from EmtptyInfo
impl FromStr for EndpointInfo {
    type Err = ();
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.try_into()?)
    }
}
