use std::usize;

use crate::{animation::AnimationTarget, id::FixtureId};

use super::{context::WorldContext, Coord};

#[derive(Clone)]
pub struct Fixture {
    id: FixtureId,
    num_leds: u32, // Assumes 1 output per fixture
    led_positions: Vec<Coord>,
}

pub struct FixtureProps {
    pub num_leds: u32,
    pub led_positions: Vec<Coord>,
}

enum FixtureError {
    SetLEDPosition(FixtureErrorSetLEDPosition),
}

pub enum FixtureErrorSetLEDPosition {
    LEDDoesNotExist,
}

impl Fixture {
    pub fn new(id: FixtureId, props: FixtureProps) -> Fixture {
        Fixture {
            id,
            num_leds: props.num_leds,
            led_positions: props.led_positions,
        }
    }

    pub fn id(&self) -> &FixtureId {
        &self.id
    }

    pub fn led_count(&self) -> u32 {
        self.num_leds
    }

    pub fn set_led_position(
        &mut self,
        led: u32,
        position: Coord,
    ) -> Result<(), FixtureErrorSetLEDPosition> {
        if led as usize >= self.led_positions.len() {
            return Err(FixtureErrorSetLEDPosition::LEDDoesNotExist);
        }

        self.led_positions[led as usize] = position;

        Ok(())
    }

    pub fn get_led_position(&self, led: u32) -> Option<&Coord> {
        self.led_positions.get(led as usize)
    }

    pub fn get_all_led_positions(&self) -> Vec<Coord> {
        self.led_positions.clone()
    }
}

impl AnimationTarget for Fixture {
    fn num_leds(&self, _context: &WorldContext) -> u32 {
        self.led_count()
    }

    fn led_positions(&self, _context: &WorldContext) -> Vec<Coord> {
        self.get_all_led_positions()
    }
}
