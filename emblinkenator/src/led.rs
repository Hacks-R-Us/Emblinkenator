#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone)]
pub struct LED {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl LED {
    pub fn flat_u32(&self) -> Vec<u32> {
        vec![u32::from(self.r), u32::from(self.g), u32::from(self.b)]
    }

    pub fn flat_u8(&self) -> Vec<u8> {
        vec![self.r, self.g, self.b]
    }
}

impl From<&[u32]> for LED {
    fn from(value: &[u32]) -> LED {
        let r: u8 = (*value.get(0).unwrap_or(&0)).clamp(0, 255) as u8;
        let g: u8 = (*value.get(1).unwrap_or(&0)).clamp(0, 255) as u8;
        let b: u8 = (*value.get(2).unwrap_or(&0)).clamp(0, 255) as u8;

        LED { r, g, b }
    }
}

impl Default for LED {
    fn default() -> Self {
        LED { r: 0, g: 0, b: 0 }
    }
}
