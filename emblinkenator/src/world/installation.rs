use std::collections::HashMap;

use crate::{
    animation::AnimationTarget,
    id::{FixtureId, InstallationId},
};

use super::{context::WorldContext, Coord};

#[derive(Clone)]
pub struct Installation {
    id: InstallationId,
    fixtures: HashMap<FixtureId, Coord>,
    fixture_order: Vec<FixtureId>,
}

pub struct InstallationProps {
    fixtures: HashMap<FixtureId, Coord>,
    fixture_order: Vec<FixtureId>,
}

pub struct FixtureOrder {
    id: FixtureId,
    position: Coord,
}

impl Installation {
    fn new(id: InstallationId) -> Installation {
        Installation {
            id,
            fixtures: HashMap::new(),
            fixture_order: vec![],
        }
    }

    pub fn id(&self) -> &InstallationId {
        &self.id
    }

    pub fn set_fixtures(&mut self, order: Vec<FixtureOrder>) {
        self.fixtures = HashMap::new();
        self.fixture_order = vec![];

        for fixture in order.iter() {
            self.fixtures.insert(fixture.id.clone(), fixture.position);
            self.fixture_order.push(fixture.id.clone());
        }
    }

    pub fn get_fixture_position(&self, fixture_id: &FixtureId) -> Option<&Coord> {
        self.fixtures.get(fixture_id)
    }

    pub fn get_led_position(&self, context: &WorldContext, led: u32) -> Coord {
        let mut total = 0;

        for fixture_id in self.fixture_order.iter() {
            let fixture = context.get_fixture(fixture_id);

            if let Some(fixture) = fixture {
                if total + fixture.led_count() < led {
                    return *fixture
                        .get_led_position(led - total)
                        .unwrap_or(&Coord::origin());
                }

                total += fixture.led_count();
            }
        }

        // Fall back to 0,0,0 so all LEDs have a valid position.
        Coord::origin()
    }

    pub fn led_count(&self, context: &WorldContext) -> u32 {
        let mut led_count = 0;

        for fixture_id in self.fixture_order.iter() {
            let fixture = context.get_fixture(fixture_id);

            if let Some(fixture) = fixture {
                led_count += fixture.led_count();
            }
        }

        led_count
    }

    pub fn get_all_led_positions(&self, context: &WorldContext) -> Vec<Coord> {
        let mut positions: Vec<Coord> = vec![];

        for fixture_id in self.fixture_order.iter() {
            if let Some(fixture) = context.get_fixture(fixture_id) {
                positions.append(&mut fixture.get_all_led_positions());
            }
        }

        positions
    }

    pub fn get_fixture_chunks(&self, context: &WorldContext) -> Vec<(FixtureId, u32)> {
        let mut result: Vec<(FixtureId, u32)> = vec![];

        for fixture_id in self.fixture_order.iter() {
            if let Some(fixture) = context.get_fixture(fixture_id) {
                result.push((fixture_id.clone(), fixture.led_count()))
            }
        }

        result
    }
}

impl AnimationTarget for Installation {
    fn num_leds(&self, context: &WorldContext) -> u32 {
        self.led_count(context)
    }

    fn led_positions(&self, context: &WorldContext) -> Vec<Coord> {
        self.get_all_led_positions(context)
    }
}
