use std::fmt;
use std::str::FromStr;

use crate::rmw::rmw_qos_durability_policy_e_RMW_QOS_POLICY_DURABILITY_SYSTEM_DEFAULT as DURABILITY_SYSTEM_DEFAULT;
use crate::rmw::rmw_qos_durability_policy_e_RMW_QOS_POLICY_DURABILITY_UNKNOWN as DURABILITY_UNKNOWN;
use crate::rmw::rmw_qos_history_policy_e_RMW_QOS_POLICY_HISTORY_SYSTEM_DEFAULT as HISTORY_SYSTEM_DEFAULT;
use crate::rmw::rmw_qos_history_policy_e_RMW_QOS_POLICY_HISTORY_UNKNOWN as HISTORY_UNKNOWN;
use crate::rmw::rmw_qos_liveliness_policy_e_RMW_QOS_POLICY_LIVELINESS_SYSTEM_DEFAULT as LIVELINESS_SYSTEM_DEFAULT;
use crate::rmw::rmw_qos_liveliness_policy_e_RMW_QOS_POLICY_LIVELINESS_UNKNOWN as POLICY_LIVELINESS_UNKNOWN;
use crate::rmw::rmw_qos_profile_t;
use crate::rmw::rmw_qos_reliability_policy_e_RMW_QOS_POLICY_RELIABILITY_SYSTEM_DEFAULT as RELIABILITY_SYSTEM_DEFAULT;
use crate::rmw::rmw_qos_reliability_policy_e_RMW_QOS_POLICY_RELIABILITY_UNKNOWN as RELIABILITY_UNKNOWN;
use crate::rmw::RMW_QOS_POLICY_DEPTH_SYSTEM_DEFAULT as DEPTH_SYSTEM_DEFAULT;
use crate::DEFAULT_QOS;

// Implement additional functionality for `rmw_qos_profile_t`
impl rmw_qos_profile_t {
    // Validates the QoS profile by checking for invalid policies.
    pub fn is_valid(&self) -> bool {
        self.history < HISTORY_UNKNOWN
            && self.reliability < RELIABILITY_UNKNOWN
            && self.durability < DURABILITY_UNKNOWN
            && self.liveliness < POLICY_LIVELINESS_UNKNOWN
    }
    // Sets the QoS profile to default values for any system default settings.
    pub fn set_default_profile(&mut self) {
        if self.history == HISTORY_SYSTEM_DEFAULT {
            self.history = DEFAULT_QOS.history;
        }
        if self.depth == DEPTH_SYSTEM_DEFAULT as usize {
            self.depth = DEFAULT_QOS.depth;
        }
        if self.reliability == RELIABILITY_SYSTEM_DEFAULT {
            self.reliability = DEFAULT_QOS.reliability;
        }
        if self.durability == DURABILITY_SYSTEM_DEFAULT {
            self.durability = DEFAULT_QOS.durability;
        }
        if self.deadline.sec == 0 && self.deadline.nsec == 0 {
            self.deadline = DEFAULT_QOS.deadline;
        }
        if self.lifespan.sec == 0 && self.lifespan.nsec == 0 {
            self.lifespan = DEFAULT_QOS.lifespan;
        }
        if self.liveliness == LIVELINESS_SYSTEM_DEFAULT {
            self.liveliness = DEFAULT_QOS.liveliness;
        }
        if self.liveliness_lease_duration.sec == 0 && self.liveliness_lease_duration.nsec == 0 {
            self.liveliness_lease_duration = DEFAULT_QOS.liveliness_lease_duration;
        }
    }
}
// Implement the `Display` trait for `rmw_qos_profile_t` to format it as a string
impl fmt::Display for rmw_qos_profile_t {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn keyexpr<T: PartialEq + ToString>(current: &T, default: &T) -> String {
            if current != default {
                current.to_string()
            } else {
                "".to_string()
            }
        }
        let default_qos = DEFAULT_QOS;
        write!(
            f,
            "{}:{}:{},{}:{},{}:{},{}:{},{},{}",
            keyexpr(&self.reliability, &default_qos.reliability),
            keyexpr(&self.durability, &default_qos.durability),
            keyexpr(&self.history, &default_qos.history),
            keyexpr(&self.depth, &default_qos.depth),
            keyexpr(&self.deadline.sec, &default_qos.deadline.sec),
            keyexpr(&self.deadline.nsec, &default_qos.deadline.nsec),
            keyexpr(&self.lifespan.sec, &default_qos.lifespan.sec),
            keyexpr(&self.lifespan.nsec, &default_qos.lifespan.nsec),
            keyexpr(&self.liveliness, &default_qos.liveliness),
            keyexpr(
                &self.liveliness_lease_duration.sec,
                &default_qos.liveliness_lease_duration.sec
            ),
            keyexpr(
                &self.liveliness_lease_duration.nsec,
                &default_qos.liveliness_lease_duration.nsec
            ),
        )?;
        Ok(())
    }
}
// Implement `TryFrom<&str>` for `rmw_qos_profile_t` to parse QoS profiles from strings
impl TryFrom<&str> for rmw_qos_profile_t {
    type Error = &'static str;
    fn try_from(key_expr: &str) -> Result<Self, Self::Error> {
        fn split_and_check_length<'a>(
            string: &'a str,
            delimiter: &'a str,
            length: usize,
        ) -> Result<Vec<&'a str>, <rmw_qos_profile_t as TryFrom<&'static str>>::Error> {
            let items: Vec<&str> = string.split(delimiter).collect();
            if items.len() == length {
                Ok(items)
            } else {
                Err("Invalid length")
            }
        }

        fn parse_value_if_exists<T: FromStr>(dst: &mut T, src: &str) {
            if let Ok(value) = src.parse::<T>() {
                *dst = value;
            }
        }

        let mut qos = DEFAULT_QOS;
        let parts = split_and_check_length(key_expr, ":", 6)?;
        let history_parts = split_and_check_length(parts[2], ",", 2)?;
        let deadline_parts = split_and_check_length(parts[3], ",", 2)?;
        let lifespan_parts = split_and_check_length(parts[4], ",", 2)?;
        let liveliness_parts = split_and_check_length(parts[5], ",", 3)?;
        parse_value_if_exists(&mut qos.reliability, parts[0]);
        parse_value_if_exists(&mut qos.durability, parts[1]);
        parse_value_if_exists(&mut qos.history, history_parts[0]);
        parse_value_if_exists(&mut qos.depth, history_parts[1]);
        parse_value_if_exists(&mut qos.deadline.sec, deadline_parts[0]);
        parse_value_if_exists(&mut qos.deadline.nsec, deadline_parts[1]);
        parse_value_if_exists(&mut qos.lifespan.sec, lifespan_parts[0]);
        parse_value_if_exists(&mut qos.lifespan.nsec, lifespan_parts[1]);
        parse_value_if_exists(&mut qos.liveliness, liveliness_parts[0]);
        parse_value_if_exists(&mut qos.liveliness_lease_duration.sec, liveliness_parts[1]);
        parse_value_if_exists(&mut qos.liveliness_lease_duration.nsec, liveliness_parts[2]);
        Ok(qos)
    }
}
// Implements FromStr for rmw_qos_profile_t, delegating to TryFrom<&str>
impl FromStr for rmw_qos_profile_t {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.try_into()?)
    }
}
