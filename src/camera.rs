use cgmath::{Vector3, Zero, Matrix4, Vector2, Rad, Matrix3};
use glutin::event::VirtualKeyCode;
use std::time::Duration;

pub struct Camera {
    position: Vector3<f32>,
    yaw: f32,
    pitch: f32,
    forwards: bool,
    backwards: bool,
    leftwards: bool,
    rightwards: bool,
    up: bool,
    down: bool,
    speed: f32
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vector3::zero() + Vector3::new(0.0, 1.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            forwards: false,
            backwards: false,
            leftwards: false,
            rightwards: false,
            up: false,
            down: false,
            speed: 0.0
        }
    }
    
    pub fn mouse_movement(&mut self, delta: Vector2<f32>) {
        const SENSITIVITY: f32 = 0.005;
        
        self.yaw -= delta.x * SENSITIVITY;
        self.pitch -= delta.y * SENSITIVITY;
    }
    
    pub fn scroll_wheel(&mut self, delta: f32) {
        const SENSITIVITY: f32 = 0.2;
        
        self.speed += delta * SENSITIVITY;
    }
    
    pub fn key_pressed(&mut self, code: VirtualKeyCode) {
        if let Some(field) = self.key_field_mut(code) {
            *field = true;
        }
    }
    
    pub fn key_released(&mut self, code: VirtualKeyCode) {
        if let Some(field) = self.key_field_mut(code) {
            *field = false;
        }
    }
    
    fn key_field_mut(&mut self, code: VirtualKeyCode) -> Option<&mut bool> {
        match code {
            VirtualKeyCode::Z => Some(&mut self.forwards),
            VirtualKeyCode::S => Some(&mut self.backwards),
            VirtualKeyCode::Q => Some(&mut self.leftwards),
            VirtualKeyCode::D => Some(&mut self.rightwards),
            VirtualKeyCode::Space => Some(&mut self.up),
            VirtualKeyCode::LShift => Some(&mut self.down),
            _ => None
        }
    }
    
    pub fn update_position(&mut self, delta_time: Duration) {
        let speed = self.speed.exp();
        let seconds = delta_time.as_secs_f32();
        let horizontal_rotation = Matrix3::from_angle_y(Rad(self.yaw));
        let direction = horizontal_rotation * Vector3::new(
            axis_unit(self.leftwards, self.rightwards),
            axis_unit(self.down, self.up),
            axis_unit(self.forwards, self.backwards)
        );
        
        let delta = (seconds * speed) * direction;
        self.position += delta;
    }
    
    pub fn get_transformation(&self) -> Matrix4<f32> {
        Matrix4::from_translation(self.position) * 
            Matrix4::from_angle_y(Rad(self.yaw)) *
            Matrix4::from_angle_x(Rad(self.pitch))
    }
}

fn axis_unit(negative: bool, positive: bool) -> f32 {
    match (negative, positive) {
        (false, false) => 0.0,
        (false, true) => 1.0,
        (true, false) => -1.0,
        (true, true) => 0.0
    }
}