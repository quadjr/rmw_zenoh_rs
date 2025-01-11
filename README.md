# rmw_zenoh_rs

An experimental implementation of the ROS 2 RMW (ROS Middleware) layer based on Zenoh.

This project was created as part of a learning exercise for Rust. The implementation is heavily inspired by [rmw_zenoh](https://github.com/ros2/rmw_zenoh).  
Currently, it only works with ROS 2 Humble.
Excluding the unimplemented Events and Content Filtering, this implementation has passed most of RMW tests.

---

## Requirements

- [ROS 2 Humble](https://docs.ros.org/en/humble/index.html)

---

## Setup

### 1. Run a Docker container
```bash
docker run -it --net=host --name rmw_zenoh_rs_test osrf/ros:humble-desktop bash
```

### 2. Build `rmw_zenoh_rs`
```bash
# Update ROS dependencies
rosdep update
apt update

# Set up the workspace
mkdir -p ~/ws_rmw_zenoh_rs/src && cd ~/ws_rmw_zenoh_rs/src
git clone https://github.com/quadjr/rmw_zenoh_rs.git

# Install dependencies
cd ~/ws_rmw_zenoh_rs
rosdep install --from-paths src --ignore-src --rosdistro humble -y

# Build the project
source /opt/ros/humble/setup.bash
colcon build --cmake-args -DCMAKE_BUILD_TYPE=Release
```

---

## Test

### 1. Source the built workspace
Before running any commands, make sure the workspace is sourced:
```bash
cd ~/ws_rmw_zenoh_rs
source install/setup.bash
```

### 2. Terminate existing ROS 2 daemons
Stop any ROS 2 daemon processes started with a different RMW:
```bash
pkill -9 -f ros && ros2 daemon stop
```

This step is necessary because ROS 2 CLI commands (e.g., `ros2 node list`) may not function correctly if the daemon was started with a different RMW.

### 3. Run the `talker` node
In the first terminal:
```bash
docker exec -it rmw_zenoh_rs_test bash
cd ~/ws_rmw_zenoh_rs
source install/setup.bash
export RMW_IMPLEMENTATION=rmw_zenoh_rs
ros2 run demo_nodes_cpp talker
```

### 4. Run the `listener` node
In a second terminal:
```bash
docker exec -it rmw_zenoh_rs_test bash
cd ~/ws_rmw_zenoh_rs
source install/setup.bash
export RMW_IMPLEMENTATION=rmw_zenoh_rs
ros2 run demo_nodes_cpp listener
```

The `listener` node should start receiving messages on the `/chatter` topic.

---

## Configuration

### Zenoh Configuration
The default Zenoh configuration file is located at `config/DEFAULT_RMW_ZENOH_SESSION_CONFIG.json5`.

To use a custom configuration file, set the `ZENOH_SESSION_CONFIG_URI` environment variable. For example:
```bash
export ZENOH_SESSION_CONFIG_URI=$HOME/MY_ZENOH_SESSION_CONFIG.json5
```
scouting.multicast.interface is overwritten with the loopback interface when the environment variable ROS_LOCALHOST_ONLY is set to 1.

---

## Logging

Logging functionality has not been implemented yet.

---

## Known Issues

- **Events**: Events has not been implemented yet.
- **Content filtering**: Content filtering has not been implemented yet.
