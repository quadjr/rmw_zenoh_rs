use ament_rs::Ament;
use get_if_addrs::get_if_addrs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use zenoh::Wait;

use crate::rmw::rcutils_allocator_t;

// The Context struct represents the execution context of the middleware
pub struct Context {
    next_node_id: AtomicUsize,
    pub session: zenoh::Session,
    pub domain_id: usize,
    pub enclave: String,
    pub allocator: rcutils_allocator_t,
    pub wait_set_cv: Arc<(Mutex<()>, Condvar)>,
}

impl Context {
    // Constructor function to create a new Context
    pub fn new(
        domain_id: usize,
        localhost_only: bool,
        enclave: &str,
        allocator: rcutils_allocator_t,
    ) -> Result<Self, ()> {
        // Attempt to get the path to the configuration file
        let config_path = Self::get_config_path()?;
        // Load the Zenoh configuration from the file
        let mut config = zenoh::Config::from_file(config_path).map_err(|_| ())?;
        if localhost_only {
            // If localhost-only is enabled, find the loopback network interface
            let loopback_if = get_if_addrs()
                .map_err(|_| ())?
                .into_iter()
                .find(|iface| iface.is_loopback())
                .ok_or(())?;
            // Configure Zenoh to use the loopback interface
            config
                .scouting
                .multicast
                .set_interface(Some(loopback_if.name))
                .map_err(|_| ())?;
        }
        // Open a Zenoh session with the configured settings
        let session = zenoh::open(config).wait().map_err(|_| ())?;
        Ok(Self {
            next_node_id: AtomicUsize::new(0),
            session,
            domain_id,
            enclave: enclave.to_string(),
            allocator,
            wait_set_cv: Arc::new((Mutex::new(()), Condvar::new())),
        })
    }

    // Retrieves the path to the Zenoh configuration file
    fn get_config_path() -> Result<PathBuf, ()> {
        // Create a new Ament instance for locating package directories
        let ament = Ament::new().map_err(|_| ())?;
        let mut config_path = PathBuf::new();
        // Append the share directory of the `rmw_zenoh_rs` package to the path
        config_path.push(
            ament
                .get_package_share_directory("rmw_zenoh_rs")
                .unwrap_or(".".to_string().into()),
        );
        config_path.push("config");
        // Append the configuration file name from the environment variable or use a default value
        config_path.push(
            std::env::var("ZENOH_SESSION_CONFIG_URI")
                .unwrap_or("DEFAULT_RMW_ZENOH_SESSION_CONFIG.json5".to_string()),
        );
        Ok(config_path)
    }

    // Generates a unique node ID by incrementing the counter atomically
    pub fn generate_node_id(&mut self) -> usize {
        return self.next_node_id.fetch_add(1, Ordering::Relaxed);
    }
}
