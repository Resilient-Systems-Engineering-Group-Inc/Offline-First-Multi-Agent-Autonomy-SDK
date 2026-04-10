//! Visualization for digital twins.
//!
//! This module provides 2D/3D visualization capabilities for digital twin entities,
//! including scene management, camera control, and rendering backends.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::Error;
use crate::model::{Entity, EntityId, EntityState, EntityType, Property};
use crate::physics::{CollisionShape, PhysicalProperties};

/// Camera projection type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Orthographic projection (2D).
    Orthographic,
    /// Perspective projection (3D).
    Perspective {
        /// Field of view in radians.
        fov: f64,
        /// Aspect ratio (width / height).
        aspect: f64,
        /// Near clipping plane.
        near: f64,
        /// Far clipping plane.
        far: f64,
    },
}

/// Camera view configuration.
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world coordinates (x, y, z).
    pub position: (f64, f64, f64),
    /// Camera target (look-at point).
    pub target: (f64, f64, f64),
    /// Up vector (usually (0, 1, 0)).
    pub up: (f64, f64, f64),
    /// Projection type.
    pub projection: Projection,
    /// Viewport width in pixels.
    pub viewport_width: u32,
    /// Viewport height in pixels.
    pub viewport_height: u32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: (10.0, 10.0, 10.0),
            target: (0.0, 0.0, 0.0),
            up: (0.0, 1.0, 0.0),
            projection: Projection::Perspective {
                fov: 60.0f64.to_radians(),
                aspect: 16.0 / 9.0,
                near: 0.1,
                far: 1000.0,
            },
            viewport_width: 800,
            viewport_height: 600,
        }
    }
}

impl Camera {
    /// Create a new camera with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set orthographic projection.
    pub fn orthographic(mut self, left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) -> Self {
        // For orthographic, we store bounds in a custom way; for simplicity we'll use a placeholder.
        // In a real implementation, you'd store these parameters.
        self.projection = Projection::Orthographic;
        self
    }

    /// Move the camera to a new position.
    pub fn move_to(&mut self, x: f64, y: f64, z: f64) {
        self.position = (x, y, z);
    }

    /// Look at a target point.
    pub fn look_at(&mut self, target_x: f64, target_y: f64, target_z: f64) {
        self.target = (target_x, target_y, target_z);
    }

    /// Compute view matrix (simplified).
    pub fn view_matrix(&self) -> [[f64; 4]; 4] {
        // Simplified identity matrix; in real implementation use glam or similar.
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    /// Compute projection matrix (simplified).
    pub fn projection_matrix(&self) -> [[f64; 4]; 4] {
        match self.projection {
            Projection::Orthographic => [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            Projection::Perspective { fov, aspect, near, far } => {
                // Simplified perspective matrix
                let f = 1.0 / (fov / 2.0).tan();
                [
                    [f / aspect, 0.0, 0.0, 0.0],
                    [0.0, f, 0.0, 0.0],
                    [0.0, 0.0, (far + near) / (near - far), -1.0],
                    [0.0, 0.0, (2.0 * far * near) / (near - far), 0.0],
                ]
            }
        }
    }
}

/// Visual representation of an entity.
#[derive(Debug, Clone)]
pub struct VisualRepresentation {
    /// Entity ID.
    pub entity_id: EntityId,
    /// Mesh data (vertices, indices, normals, etc.).
    /// In a real implementation, this would be a proper mesh structure.
    pub mesh: Option<Vec<f32>>,
    /// Texture ID or path.
    pub texture: Option<String>,
    /// Color (RGBA) if no texture.
    pub color: (f32, f32, f32, f32),
    /// Scale factor.
    pub scale: (f32, f32, f32),
    /// Whether the entity is visible.
    pub visible: bool,
    /// Layer (for 2D rendering).
    pub layer: u32,
}

impl VisualRepresentation {
    /// Create a simple box representation.
    pub fn simple_box(entity_id: EntityId, color: (f32, f32, f32, f32)) -> Self {
        Self {
            entity_id,
            mesh: None,
            texture: None,
            color,
            scale: (1.0, 1.0, 1.0),
            visible: true,
            layer: 0,
        }
    }

    /// Create a sphere representation.
    pub fn simple_sphere(entity_id: EntityId, color: (f32, f32, f32, f32)) -> Self {
        Self {
            entity_id,
            mesh: None,
            texture: None,
            color,
            scale: (1.0, 1.0, 1.0),
            visible: true,
            layer: 0,
        }
    }
}

/// Scene containing entities and their visual representations.
#[derive(Debug, Clone)]
pub struct Scene {
    /// Map from entity ID to visual representation.
    visuals: HashMap<EntityId, VisualRepresentation>,
    /// Background color (RGBA).
    pub background_color: (f32, f32, f32, f32),
    /// Ambient light color and intensity.
    pub ambient_light: (f32, f32, f32, f32),
    /// Directional light direction and color.
    pub directional_light: Option<((f32, f32, f32), (f32, f32, f32))>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            visuals: HashMap::new(),
            background_color: (0.1, 0.1, 0.1, 1.0),
            ambient_light: (0.2, 0.2, 0.2, 1.0),
            directional_light: Some(((1.0, -1.0, 0.5).into(), (1.0, 1.0, 0.9).into())),
        }
    }
}

impl Scene {
    /// Create a new empty scene.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a visual representation to the scene.
    pub fn add_visual(&mut self, visual: VisualRepresentation) {
        self.visuals.insert(visual.entity_id, visual);
    }

    /// Remove a visual representation by entity ID.
    pub fn remove_visual(&mut self, entity_id: EntityId) {
        self.visuals.remove(&entity_id);
    }

    /// Get visual representation for an entity.
    pub fn get_visual(&self, entity_id: EntityId) -> Option<&VisualRepresentation> {
        self.visuals.get(&entity_id)
    }

    /// Update visual representation based on entity state.
    pub fn update_from_state(&mut self, state: &EntityState) {
        if let Some(visual) = self.visuals.get_mut(&state.entity_id) {
            // In a real implementation, update position, orientation, etc.
            // For now, just mark as visible if status is operational.
            visual.visible = state.status.is_operational();
        }
    }

    /// Clear all visuals.
    pub fn clear(&mut self) {
        self.visuals.clear();
    }

    /// Number of visuals in the scene.
    pub fn len(&self) -> usize {
        self.visuals.len()
    }

    /// Check if scene is empty.
    pub fn is_empty(&self) -> bool {
        self.visuals.is_empty()
    }
}

/// Trait for rendering backends.
pub trait Renderer: Send + Sync {
    /// Initialize the renderer.
    fn init(&mut self) -> Result<(), Error>;

    /// Render a scene with a given camera.
    fn render(&mut self, scene: &Scene, camera: &Camera) -> Result<(), Error>;

    /// Resize the viewport.
    fn resize(&mut self, width: u32, height: u32);

    /// Capture the current frame as image bytes (RGBA).
    fn capture_frame(&self) -> Result<Vec<u8>, Error>;

    /// Clean up resources.
    fn cleanup(&mut self);
}

/// Simple 2D renderer for debugging (uses CPU drawing).
#[derive(Debug)]
pub struct Simple2DRenderer {
    /// Framebuffer width.
    width: u32,
    /// Framebuffer height.
    height: u32,
    /// Framebuffer pixels (RGBA).
    framebuffer: Vec<u8>,
}

impl Simple2DRenderer {
    /// Create a new 2D renderer with given dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        let mut framebuffer = vec![0; size];
        // Set background to dark gray
        for i in 0..(width * height) as usize {
            framebuffer[i * 4] = 30;     // R
            framebuffer[i * 4 + 1] = 30; // G
            framebuffer[i * 4 + 2] = 30; // B
            framebuffer[i * 4 + 3] = 255; // A
        }
        Self {
            width,
            height,
            framebuffer,
        }
    }

    /// Draw a rectangle.
    fn draw_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: (u8, u8, u8, u8)) {
        let (r, g, b, a) = color;
        for dy in 0..h {
            let row = y + dy;
            if row >= self.height {
                continue;
            }
            for dx in 0..w {
                let col = x + dx;
                if col >= self.width {
                    continue;
                }
                let idx = ((row * self.width + col) * 4) as usize;
                self.framebuffer[idx] = r;
                self.framebuffer[idx + 1] = g;
                self.framebuffer[idx + 2] = b;
                self.framebuffer[idx + 3] = a;
            }
        }
    }

    /// Draw a circle.
    fn draw_circle(&mut self, cx: u32, cy: u32, radius: u32, color: (u8, u8, u8, u8)) {
        let (r, g, b, a) = color;
        let r_sq = (radius * radius) as i32;
        let start_x = cx.saturating_sub(radius);
        let end_x = (cx + radius).min(self.width - 1);
        let start_y = cy.saturating_sub(radius);
        let end_y = (cy + radius).min(self.height - 1);
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let dx = x as i32 - cx as i32;
                let dy = y as i32 - cy as i32;
                if dx * dx + dy * dy <= r_sq {
                    let idx = ((y * self.width + x) * 4) as usize;
                    self.framebuffer[idx] = r;
                    self.framebuffer[idx + 1] = g;
                    self.framebuffer[idx + 2] = b;
                    self.framebuffer[idx + 3] = a;
                }
            }
        }
    }
}

impl Renderer for Simple2DRenderer {
    fn init(&mut self) -> Result<(), Error> {
        // Nothing to initialize for CPU renderer.
        Ok(())
    }

    fn render(&mut self, scene: &Scene, camera: &Camera) -> Result<(), Error> {
        // Clear framebuffer
        for pixel in self.framebuffer.chunks_mut(4) {
            pixel[0] = 30;
            pixel[1] = 30;
            pixel[2] = 30;
            pixel[3] = 255;
        }

        // Draw each visual as a simple shape
        for visual in scene.visuals.values() {
            if !visual.visible {
                continue;
            }
            // Map entity ID to position (simplified: use hash to position)
            let hash = visual.entity_id.as_u128();
            let x = ((hash % 100) as u32) * 10 + 50;
            let y = ((hash / 100 % 100) as u32) * 10 + 50;
            let color = (
                (visual.color.0 * 255.0) as u8,
                (visual.color.1 * 255.0) as u8,
                (visual.color.2 * 255.0) as u8,
                (visual.color.3 * 255.0) as u8,
            );
            match visual.mesh {
                Some(_) => self.draw_rect(x, y, 20, 20, color),
                None => self.draw_circle(x, y, 10, color),
            }
        }
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) {
        let size = (width * height * 4) as usize;
        self.framebuffer.resize(size, 0);
        self.width = width;
        self.height = height;
    }

    fn capture_frame(&self) -> Result<Vec<u8>, Error> {
        Ok(self.framebuffer.clone())
    }

    fn cleanup(&mut self) {
        // Nothing to clean up.
    }
}

/// Visualization manager that coordinates scene, camera, and renderer.
pub struct VisualizationManager<R: Renderer> {
    /// Scene being visualized.
    scene: Scene,
    /// Active camera.
    camera: Camera,
    /// Renderer backend.
    renderer: R,
    /// Whether visualization is running.
    running: bool,
}

impl<R: Renderer> VisualizationManager<R> {
    /// Create a new visualization manager.
    pub fn new(renderer: R) -> Self {
        Self {
            scene: Scene::new(),
            camera: Camera::new(),
            renderer,
            running: false,
        }
    }

    /// Initialize the visualization system.
    pub fn init(&mut self) -> Result<(), Error> {
        self.renderer.init()?;
        self.running = true;
        Ok(())
    }

    /// Render a single frame.
    pub fn render_frame(&mut self) -> Result<(), Error> {
        if !self.running {
            return Err(Error::Other("Visualization not initialized".into()));
        }
        self.renderer.render(&self.scene, &self.camera)
    }

    /// Update scene from a list of entity states.
    pub fn update_scene(&mut self, states: &[EntityState]) {
        for state in states {
            self.scene.update_from_state(state);
        }
    }

    /// Add an entity to the scene with default visual.
    pub fn add_entity(&mut self, entity: &Entity, state: &EntityState) {
        let color = match entity.entity_type {
            EntityType::Robot => (0.0, 0.8, 0.0, 1.0), // Green
            EntityType::Sensor => (0.0, 0.5, 1.0, 1.0), // Blue
            EntityType::Actuator => (1.0, 0.5, 0.0, 1.0), // Orange
            EntityType::Room => (0.5, 0.5, 0.5, 1.0), // Gray
            EntityType::Machine => (0.8, 0.0, 0.8, 1.0), // Purple
            EntityType::Human => (1.0, 0.8, 0.6, 1.0), // Skin tone
            EntityType::Virtual => (0.5, 0.5, 0.0, 1.0), // Yellowish
            EntityType::Custom(_) => (0.7, 0.7, 0.7, 1.0),
        };
        let visual = VisualRepresentation::simple_box(entity.id, color);
        self.scene.add_visual(visual);
        self.scene.update_from_state(state);
    }

    /// Remove an entity from the scene.
    pub fn remove_entity(&mut self, entity_id: EntityId) {
        self.scene.remove_visual(entity_id);
    }

    /// Get mutable reference to camera.
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// Get reference to camera.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Get reference to scene.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Get mutable reference to scene.
    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    /// Capture current frame as image bytes.
    pub fn capture_frame(&self) -> Result<Vec<u8>, Error> {
        self.renderer.capture_frame()
    }

    /// Stop visualization and clean up.
    pub fn stop(&mut self) -> Result<(), Error> {
        self.renderer.cleanup();
        self.running = false;
        Ok(())
    }
}

/// Helper function to create a default 2D visualization manager.
pub fn create_2d_visualization(width: u32, height: u32) -> VisualizationManager<Simple2DRenderer> {
    let renderer = Simple2DRenderer::new(width, height);
    VisualizationManager::new(renderer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_camera_default() {
        let camera = Camera::default();
        assert_eq!(camera.position, (10.0, 10.0, 10.0));
        assert_eq!(camera.target, (0.0, 0.0, 0.0));
        assert_eq!(camera.up, (0.0, 1.0, 0.0));
        assert!(matches!(camera.projection, Projection::Perspective { .. }));
    }

    #[test]
    fn test_scene_add_remove() {
        let mut scene = Scene::new();
        let entity_id = Uuid::new_v4();
        let visual = VisualRepresentation::simple_box(entity_id, (1.0, 0.0, 0.0, 1.0));
        scene.add_visual(visual);
        assert_eq!(scene.len(), 1);
        assert!(scene.get_visual(entity_id).is_some());
        scene.remove_visual(entity_id);
        assert!(scene.get_visual(entity_id).is_none());
        assert!(scene.is_empty());
    }

    #[test]
    fn test_simple_2d_renderer() {
        let mut renderer = Simple2DRenderer::new(100, 100);
        assert!(renderer.init().is_ok());
        let scene = Scene::new();
        let camera = Camera::new();
        assert!(renderer.render(&scene, &camera).is_ok());
        let frame = renderer.capture_frame().unwrap();
        assert_eq!(frame.len(), 100 * 100 * 4);
        // First pixel should be background color (30,30,30,255)
        assert_eq!(frame[0], 30);
        assert_eq!(frame[1], 30);
        assert_eq!(frame[2], 30);
        assert_eq!(frame[3], 255);
    }

    #[test]
    fn test_visualization_manager() {
        let renderer = Simple2DRenderer::new(200, 200);
        let mut manager = VisualizationManager::new(renderer);
        assert!(manager.init().is_ok());
        // Can't capture before rendering
        let _ = manager.capture_frame(); // This may fail but that's okay
        assert!(manager.render_frame().is_ok());
        let frame = manager.capture_frame().unwrap();
        assert_eq!(frame.len(), 200 * 200 * 4);
        assert!(manager.stop().is_ok());
    }
}