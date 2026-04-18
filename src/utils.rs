pub struct Utils {}
impl Utils {
    pub fn normalize(v: [f32; 3]) -> [f32; 3] {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len == 0.0 {
            return [0.0, 0.0, 0.0];
        }
        [v[0] / len, v[1] / len, v[2] / len]
    }

    pub fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    pub fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    pub fn calculate_3d_audio(
        local_pos: [f32; 3],
        enemy_pos: [f32; 3],
        forward: [f32; 3],
    ) -> (f32, f32, f32) {
        let dir = Self::normalize([
            enemy_pos[0] - local_pos[0],
            enemy_pos[1] - local_pos[1],
            enemy_pos[2] - local_pos[2],
        ]);

        let world_up = [0.0, 0.0, 1.0];
        //let right = Self::normalize(Self::cross(forward, world_up));
        let right = Self::normalize(Self::cross(world_up, forward));
        let front = Self::dot(dir, forward);
        let pan = Self::dot(dir, right).clamp(-1.0, 1.0);

        let left = ((1.0 - pan) * 0.5).sqrt();
        let right = ((1.0 + pan) * 0.5).sqrt();

        let front_factor = if front > 0.0 { 1.0 } else { 0.6 };
        let distance = ((enemy_pos[0] - local_pos[0]).powi(2)
            + (enemy_pos[1] - local_pos[1]).powi(2)
            + (enemy_pos[2] - local_pos[2]).powi(2))
        .sqrt();

        let volume = (1.0 / (1.0 + distance * 0.002)).clamp(0.05, 1.0);

        (
            left * volume * front_factor,
            right * volume * front_factor,
            distance,
        )
    }
}
