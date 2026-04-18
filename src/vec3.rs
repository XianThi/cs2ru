#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }
    
    pub fn distance(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx*dx + dy*dy + dz*dz).sqrt()
    }
    
    pub fn to_screen(&self, view_matrix: &[f32; 16], screen_width: f32, screen_height: f32) -> Option<(f32, f32)> {
        let w = view_matrix[12] * self.x + view_matrix[13] * self.y + view_matrix[14] * self.z + view_matrix[15];
        if w < 0.1 { return None; }
        
        let inv_w = 1.0 / w;
        let x = (view_matrix[0] * self.x + view_matrix[1] * self.y + view_matrix[2] * self.z + view_matrix[3]) * inv_w;
        let y = (view_matrix[4] * self.x + view_matrix[5] * self.y + view_matrix[6] * self.z + view_matrix[7]) * inv_w;
        
        let screen_x = (screen_width / 2.0) * (1.0 + x);
        let screen_y = (screen_height / 2.0) * (1.0 - y);
        
        Some((screen_x, screen_y))
    }
}