use std::collections::HashMap;

use crate::{
    animation::AnimationTarget,
    id::{FixtureId, GroupId, InstallationId},
};

use super::{context::WorldContext, Coord};

// Note: Group ordering goes [...groups, ...installations]

#[derive(Clone)]
pub struct Group {
    id: GroupId,
    installations: HashMap<InstallationId, Coord>,
    installation_order: Vec<InstallationId>,
    groups: HashMap<GroupId, Coord>,
    group_order: Vec<GroupId>,
}

pub struct GroupProps {
    installations: HashMap<InstallationId, Coord>,
    installation_order: Vec<InstallationId>,
    groups: HashMap<GroupId, Coord>,
    group_order: Vec<GroupId>,
}

pub struct InstallationOrder {
    id: InstallationId,
    position: Coord,
}
pub struct GroupOrder {
    id: GroupId,
    position: Coord,
}

impl Group {
    fn new(id: GroupId) -> Group {
        Group {
            id,
            installations: HashMap::new(),
            installation_order: vec![],
            groups: HashMap::new(),
            group_order: vec![],
        }
    }

    pub fn id(&self) -> &GroupId {
        &self.id
    }

    pub fn set_installations(&mut self, order: Vec<InstallationOrder>) {
        self.installations = HashMap::new();
        self.installation_order = vec![];

        for installation in order.iter() {
            self.installations
                .insert(installation.id.clone(), installation.position);
            self.installation_order.push(installation.id.clone());
        }
    }

    pub fn set_groups(&mut self, order: Vec<GroupOrder>) {
        self.groups = HashMap::new();
        self.group_order = vec![];

        for group in order.iter() {
            self.groups.insert(group.id.clone(), group.position);
            self.group_order.push(group.id.clone());
        }
    }

    pub fn get_led_position(&self, context: &WorldContext, led: u32) -> Coord {
        let mut total = 0;
        for installation_id in self.installation_order.iter() {
            let installation = context.get_installation(installation_id);

            if let Some(installation) = installation {
                if total + installation.led_count(context) < led {
                    return installation.get_led_position(context, led - total);
                }

                total += installation.led_count(context);
            }
        }

        for group_id in self.group_order.iter() {
            let group = context.get_group(group_id);

            if let Some(group) = group {
                if total + group.led_count(context) < led {
                    return group.get_led_position(context, led - total);
                }

                total += group.led_count(context)
            }
        }

        Coord::origin()
    }

    pub fn led_count(&self, context: &WorldContext) -> u32 {
        let mut led_count = 0;

        for installation_id in self.installation_order.iter() {
            let installation = context.get_installation(installation_id);

            if let Some(installation) = installation {
                led_count += installation.led_count(context);
            }
        }

        for group_id in self.group_order.iter() {
            let group = context.get_group(group_id);

            if let Some(group) = group {
                led_count += group.led_count(context);
            }
        }

        led_count
    }

    pub fn get_all_led_positions(&self, context: &WorldContext) -> Vec<Coord> {
        let mut positions: Vec<Coord> = vec![];

        for installation_id in self.installation_order.iter() {
            if let Some(installation) = context.get_installation(installation_id) {
                positions.append(&mut installation.get_all_led_positions(context));
            }
        }

        for group_id in self.group_order.iter() {
            if let Some(group) = context.get_group(group_id) {
                positions.append(&mut group.get_all_led_positions(context));
            }
        }

        positions
    }

    pub fn get_fixture_chunks(&self, context: &WorldContext) -> Vec<(FixtureId, u32)> {
        let mut result: Vec<(FixtureId, u32)> = vec![];

        for group_id in self.group_order.iter() {
            if let Some(group) = context.get_group(group_id) {
                result.append(&mut group.get_fixture_chunks(context));
            }
        }

        for installation_id in self.installation_order.iter() {
            if let Some(installation) = context.get_installation(installation_id) {
                result.append(&mut installation.get_fixture_chunks(context));
            }
        }

        result
    }
}

impl AnimationTarget for Group {
    fn num_leds(&self, context: &WorldContext) -> u32 {
        self.led_count(context)
    }

    fn led_positions(&self, context: &WorldContext) -> Vec<Coord> {
        self.get_all_led_positions(context)
    }
}
