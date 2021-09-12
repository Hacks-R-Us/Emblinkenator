use serde::Deserialize;

pub mod context;
pub mod fixture;
pub mod group;
pub mod installation;

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Coord {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Coord {
    pub fn flat(&self) -> Vec<f32> {
        vec![self.x, self.y, self.z]
    }

    pub fn origin() -> Coord {
        Coord {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}
