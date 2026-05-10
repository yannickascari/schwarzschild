use glam::Vec3;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub ro:      [f32; 3],
    pub fov:     f32,
    pub forward: [f32; 3],
    pub _pad1:   f32,
    pub right:   [f32; 3],
    pub _pad2:   f32,
    pub up:      [f32; 3],
    pub _pad3:   f32,
}

pub struct OrbitalCamera {
    pub theta: f32,
    pub phi: f32,
    pub radius: f32,
    pub fov: f32,
}

impl OrbitalCamera {
    pub fn new() -> Self {
        Self {
            theta:  std::f32::consts::FRAC_PI_2,
            phi:    0.0,
            radius: 15.0,
            fov:    0.5,
        }
    }

    pub fn to_uniform(&self) -> CameraUniform {
        let ro = Vec3::new(
            self.radius * self.theta.sin() * self.phi.cos(),
            self.radius * self.theta.cos(),
            self.radius * self.theta.sin() * self.phi.sin(),
        );

        let forward = (-ro).normalize();

        let world_up = if self.theta.abs() < 0.1 || (self.theta - std::f32::consts::PI).abs() < 0.1 {
            Vec3::Z
        } else {
            Vec3::Y
        };

        let right = forward.cross(world_up).normalize();
        let up = right.cross(forward);

        CameraUniform {
            ro:      ro.to_array(),
            fov:     self.fov,
            forward: forward.to_array(),
            _pad1:   0.0,
            right:   right.to_array(),
            _pad2:   0.0,
            up:      up.to_array(),
            _pad3:   0.0,
        }
    }

    pub fn orbit(&mut self, delta_phi: f32, delta_theta: f32) {
        self.phi += delta_phi;

        self.theta = (self.theta + delta_theta).clamp(0.05, std::f32::consts::PI - 0.05);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.radius = (self.radius + delta).clamp(2.0, 50.0);
    }
}