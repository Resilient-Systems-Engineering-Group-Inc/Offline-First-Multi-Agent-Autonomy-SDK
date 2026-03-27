//! Device registry for managing sensors and actuators.

use crate::error::{Error, Result};
use crate::sensor::{Sensor, SensorConfig};
use crate::actuator::{Actuator, ActuatorConfig};
use crate::driver::{Driver, DriverFactory};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A registry that holds all sensors and actuators.
pub struct DeviceRegistry {
    sensors: RwLock<HashMap<String, Arc<dyn Sensor>>>,
    actuators: RwLock<HashMap<String, Arc<dyn Actuator>>>,
    drivers: RwLock<HashMap<String, Arc<dyn Driver>>>,
}

impl DeviceRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            sensors: RwLock::new(HashMap::new()),
            actuators: RwLock::new(HashMap::new()),
            drivers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a driver for a protocol.
    pub async fn register_driver(&self, protocol: String, driver: Arc<dyn Driver>) {
        self.drivers.write().await.insert(protocol, driver);
    }

    /// Add a sensor using a given protocol.
    pub async fn add_sensor(&self, config: SensorConfig, protocol: &str) -> Result<()> {
        let driver = self.get_driver(protocol).await?;
        let sensor = driver.create_sensor(config).await?;
        let id = sensor.config().id.clone();
        self.sensors.write().await.insert(id, Arc::from(sensor));
        Ok(())
    }

    /// Add an actuator using a given protocol.
    pub async fn add_actuator(&self, config: ActuatorConfig, protocol: &str) -> Result<()> {
        let driver = self.get_driver(protocol).await?;
        let actuator = driver.create_actuator(config).await?;
        let id = actuator.config().id.clone();
        self.actuators.write().await.insert(id, Arc::from(actuator));
        Ok(())
    }

    /// Get a sensor by ID.
    pub async fn get_sensor(&self, id: &str) -> Option<Arc<dyn Sensor>> {
        self.sensors.read().await.get(id).cloned()
    }

    /// Get an actuator by ID.
    pub async fn get_actuator(&self, id: &str) -> Option<Arc<dyn Actuator>> {
        self.actuators.read().await.get(id).cloned()
    }

    /// List all sensor IDs.
    pub async fn list_sensors(&self) -> Vec<String> {
        self.sensors.read().await.keys().cloned().collect()
    }

    /// List all actuator IDs.
    pub async fn list_actuators(&self) -> Vec<String> {
        self.actuators.read().await.keys().cloned().collect()
    }

    /// Remove a sensor.
    pub async fn remove_sensor(&self, id: &str) -> Result<()> {
        self.sensors.write().await.remove(id);
        Ok(())
    }

    /// Remove an actuator.
    pub async fn remove_actuator(&self, id: &str) -> Result<()> {
        self.actuators.write().await.remove(id);
        Ok(())
    }

    async fn get_driver(&self, protocol: &str) -> Result<Arc<dyn Driver>> {
        if let Some(driver) = self.drivers.read().await.get(protocol) {
            return Ok(driver.clone());
        }
        // Try to create a driver on‑the‑fly
        let driver = DriverFactory::get_driver(protocol).await?;
        let driver_arc = Arc::from(driver);
        self.drivers.write().await.insert(protocol.to_string(), driver_arc.clone());
        Ok(driver_arc)
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}