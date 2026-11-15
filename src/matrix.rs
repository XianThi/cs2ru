#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ViewMatrix {
    pub matrix: [[f32; 4]; 4],
}
impl ViewMatrix {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut matrix = [[0.0; 4]; 4];

        for i in 0..4 {
            for j in 0..4 {
                let idx = i * 4 + j;
                let offset = idx * 4;
                if offset + 4 <= bytes.len() {
                    matrix[i][j] =
                        f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap_or([0; 4]));
                }
            }
        }

        Self { matrix }
    }

    pub fn world_to_screen(
        &self,
        world_pos: & crate::vec3::Vec3,
        screen_width: f32,
        screen_height: f32,
    ) -> Option<(f32, f32)> {
        let w = self.matrix[3][0] * world_pos.x
            + self.matrix[3][1] * world_pos.y
            + self.matrix[3][2] * world_pos.z
            + self.matrix[3][3];

        if w < 0.01 {
            return None;
        }

        let inv_w = 1.0 / w;
        let x = (self.matrix[0][0] * world_pos.x
            + self.matrix[0][1] * world_pos.y
            + self.matrix[0][2] * world_pos.z
            + self.matrix[0][3])
            * inv_w;

        let y = (self.matrix[1][0] * world_pos.x
            + self.matrix[1][1] * world_pos.y
            + self.matrix[1][2] * world_pos.z
            + self.matrix[1][3])
            * inv_w;

        let screen_x = (screen_width / 2.0) * (1.0 + x);
        let screen_y = (screen_height / 2.0) * (1.0 - y);

        Some((screen_x, screen_y))
    }
}
