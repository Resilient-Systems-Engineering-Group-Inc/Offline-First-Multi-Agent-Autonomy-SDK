//! IoT sensor and actuator interfaces.
//!
//! This crate provides unified abstractions for interacting with sensors and actuators
//! across various protocols (MQTT, CoAP, Modbus, ROS2, etc.) in an offline‑first
//! multi‑agent system.

pub mod error;
pub mod sensor;
pub mod actuator;
pub mod driver;
pub mod registry;

pub use error::{Error, Result};
pub use sensor::{Sensor, SensorReading, SensorConfig};
pub use actuator::{Actuator, ActuatorCommand, ActuatorConfig};
pub use driver::{Driver, DriverFactory};
pub use registry::DeviceRegistry;

/// Pre‑import of commonly used types.
pub mod prelude {
    pub use crate::{Sensor, Actuator, DeviceRegistry};
    pub use crate::driver::{Driver, MqttDriver, CoapDriver, ModbusDriver, Ros2Driver};
}