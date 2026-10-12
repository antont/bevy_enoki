use bevy_math::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Debug, Serialize, Default)]
pub struct Rval<V>(pub V, pub f32);

impl<V> Rval<V> {
    pub fn new(value: V, randomness: f32) -> Self {
        Self(value, randomness)
    }
}

pub trait Random<V> {
    fn rand(&self) -> V;
}

impl Random<Vec2> for Rval<Vec2> {
    fn rand(&self) -> Vec2 {
        // Skip, if not needed.
        if self.1 <= 0.0001 {
            return self.0;
        }

        let max_angle = 2. * std::f32::consts::PI * self.1;
        let random_angle = (rand::random::<f32>() - 0.5) * max_angle;

        let (sin, cos) = random_angle.sin_cos();
        Vec2::new(
            self.0.x * cos - self.0.y * sin,
            self.0.x * sin + self.0.y * cos,
        )
    }
}

impl Random<f32> for Rval<f32> {
    fn rand(&self) -> f32 {
        let r = (rand::random::<f32>() - 0.5) * 2. * self.1;
        self.0 + self.0 * r
    }
}

/// An absolute random spread: `value` plus or minus up to `spread`, uniformly (cocos2d's `value + variance * rand(-1, 1)`).
/// Unlike [`Rval`], a zero value can have a spread.
#[derive(Deserialize, Clone, Copy, Debug, Serialize, Default, PartialEq)]
pub struct Spread(pub f32, pub f32);

impl Random<f32> for Spread {
    fn rand(&self) -> f32 {
        self.0 + self.1 * (rand::random::<f32>() * 2. - 1.)
    }
}
