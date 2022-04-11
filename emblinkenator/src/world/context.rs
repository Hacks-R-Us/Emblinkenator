use parking_lot::RwLock;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::broadcast::{self, Receiver, Sender};

use crate::{
    events::EventEmitter,
    id::{FixtureId, GroupId, InstallationId},
};

use super::{fixture::Fixture, group::Group, installation::Installation, Coord};

pub struct WorldContext {
    event_emitter: Sender<WorldContextEvent>,
    led_position_cache: Mutex<Option<WorldContextState>>,
    collection: WorldContextCollection,
}

pub struct WorldContextCollection {
    pub fixtures: Arc<RwLock<HashMap<FixtureId, Fixture>>>,
    pub installations: Arc<RwLock<HashMap<InstallationId, Installation>>>,
    pub groups: Arc<RwLock<HashMap<GroupId, Group>>>,
}

enum WorldContextError {
    AddFixture(WorldContextErrorAddFixture),
    RemoveFixture(WorldContextErrorRemoveFixture),
    AddInstallation(WorldContextErrorAddInstallation),
    RemoveInstallation(WorldContextErrorRemoveInstallation),
    AddGroup(WorldContextErrorAddGroup),
    RemoveGroup(WorldContextErrorRemoveGroup),
}

#[derive(Debug)]
pub enum WorldContextErrorAddFixture {
    FixtureExists,
}

#[derive(Debug)]
pub enum WorldContextErrorRemoveFixture {
    FixtureDoesNotExist,
}

#[derive(Debug)]
pub enum WorldContextErrorAddInstallation {
    InstallationExists,
}

#[derive(Debug)]
pub enum WorldContextErrorRemoveInstallation {
    InstallationDoesNotExist,
}

#[derive(Debug)]
pub enum WorldContextErrorAddGroup {
    GroupExists,
}

#[derive(Debug)]
pub enum WorldContextErrorRemoveGroup {
    GroupDoesNotExist,
}

#[derive(Clone, Debug)]
pub enum WorldContextEvent {
    FixtureAdded(FixtureId),
    FixtureRemoved(FixtureId),
    InstallationAdded(InstallationId),
    InstallationRemoved(InstallationId),
    GroupAdded(GroupId),
    GroupRemoved(GroupId),
}

#[derive(Clone)]
pub struct WorldContextState {
    pub led_positions: HashMap<String, Vec<Coord>>,
    pub num_leds: HashMap<String, u32>,
    /* Maps animation target Id to an ordered list of Fixture Id and the number of LEDs the fixture contains. */
    pub fixture_chunks: HashMap<String, Vec<(FixtureId, u32)>>,
}

impl WorldContext {
    pub fn new(collection: WorldContextCollection) -> WorldContext {
        let (tx, _) = broadcast::channel(16);
        WorldContext {
            collection,
            event_emitter: tx,
            led_position_cache: Mutex::new(None),
        }
    }

    pub fn add_fixture(&self, fixture: Fixture) -> Result<(), WorldContextErrorAddFixture> {
        let id = fixture.id().clone();

        if let Err(_err) = self
            .collection
            .fixtures
            .write()
            .try_insert(id.clone(), fixture)
        {
            return Err(WorldContextErrorAddFixture::FixtureExists);
        }

        self.on_state_changed();
        self.emit_fixture_added(id);

        Ok(())
    }

    pub fn remove_fixture(
        &mut self,
        fixture_id: &FixtureId,
    ) -> Result<(), WorldContextErrorRemoveFixture> {
        if self
            .collection
            .fixtures
            .write()
            .remove(fixture_id)
            .is_none()
        {
            return Err(WorldContextErrorRemoveFixture::FixtureDoesNotExist);
        }

        self.on_state_changed();
        self.emit_fixture_removed(fixture_id.clone());

        Ok(())
    }

    pub fn get_fixture(&self, fixture_id: &FixtureId) -> Option<Fixture> {
        self.collection.fixtures.write().get(fixture_id).cloned()
    }

    pub fn add_installation(
        &mut self,
        installation: Installation,
    ) -> Result<(), WorldContextErrorAddInstallation> {
        let id = installation.id().clone();

        if let Err(_err) = self
            .collection
            .installations
            .write()
            .try_insert(id.clone(), installation)
        {
            return Err(WorldContextErrorAddInstallation::InstallationExists);
        }

        self.on_state_changed();
        self.emit_installation_added(id);

        Ok(())
    }

    pub fn remove_installation(
        &mut self,
        installation_id: &InstallationId,
    ) -> Result<(), WorldContextErrorRemoveInstallation> {
        if self
            .collection
            .installations
            .write()
            .remove(installation_id)
            .is_none()
        {
            return Err(WorldContextErrorRemoveInstallation::InstallationDoesNotExist);
        }

        self.on_state_changed();
        self.emit_installation_removed(installation_id.clone());

        Ok(())
    }

    pub fn get_installation(&self, installation_id: &InstallationId) -> Option<Installation> {
        self.collection
            .installations
            .write()
            .get(installation_id)
            .cloned()
    }

    pub fn add_group(&mut self, group: Group) -> Result<(), WorldContextErrorAddGroup> {
        let id = group.id().clone();

        if let Err(_err) = self.collection.groups.write().try_insert(id.clone(), group) {
            return Err(WorldContextErrorAddGroup::GroupExists);
        }

        self.on_state_changed();
        self.emit_group_added(id);

        Ok(())
    }

    pub fn remove_group(&mut self, group_id: &GroupId) -> Result<(), WorldContextErrorRemoveGroup> {
        if self.collection.groups.write().remove(group_id).is_none() {
            return Err(WorldContextErrorRemoveGroup::GroupDoesNotExist);
        }

        self.on_state_changed();
        self.emit_group_removed(group_id.clone());

        Ok(())
    }

    pub fn get_group(&self, group_id: &GroupId) -> Option<Group> {
        self.collection.groups.write().get(group_id).cloned()
    }

    pub fn get_registered_fixtures(&self) -> Vec<FixtureId> {
        self.collection
            .fixtures
            .read()
            .keys()
            .map(|f| f.to_owned())
            .collect()
    }

    pub fn get_registered_installations(&self) -> Vec<InstallationId> {
        self.collection
            .installations
            .read()
            .keys()
            .map(|f| f.to_owned())
            .collect()
    }

    pub fn get_registered_groups(&self) -> Vec<GroupId> {
        self.collection
            .groups
            .read()
            .keys()
            .map(|f| f.to_owned())
            .collect()
    }

    fn on_state_changed(&self) {
        self.led_position_cache.lock().unwrap().take();
    }

    fn emit_fixture_added(&self, id: FixtureId) {
        if self.event_emitter.receiver_count() > 0 {
            self.event_emitter
                .send(WorldContextEvent::FixtureAdded(id))
                .unwrap();
        }
    }

    fn emit_installation_added(&self, id: InstallationId) {
        if self.event_emitter.receiver_count() > 0 {
            self.event_emitter
                .send(WorldContextEvent::InstallationAdded(id))
                .unwrap();
        }
    }

    fn emit_group_added(&self, id: GroupId) {
        if self.event_emitter.receiver_count() > 0 {
            self.event_emitter
                .send(WorldContextEvent::GroupAdded(id))
                .unwrap();
        }
    }

    fn emit_fixture_removed(&self, id: FixtureId) {
        if self.event_emitter.receiver_count() > 0 {
            self.event_emitter
                .send(WorldContextEvent::FixtureRemoved(id))
                .unwrap();
        }
    }

    fn emit_installation_removed(&self, id: InstallationId) {
        if self.event_emitter.receiver_count() > 0 {
            self.event_emitter
                .send(WorldContextEvent::InstallationRemoved(id))
                .unwrap();
        }
    }

    fn emit_group_removed(&self, id: GroupId) {
        if self.event_emitter.receiver_count() > 0 {
            self.event_emitter
                .send(WorldContextEvent::GroupRemoved(id))
                .unwrap();
        }
    }

    pub fn get_world_context_state(&self) -> WorldContextState {
        let cache = self.led_position_cache.lock().unwrap().clone();

        // TODO: Invalidate cache at some point somewhere
        if let Some(cache) = cache {
            return cache;
        }

        let mut led_positions: HashMap<String, Vec<Coord>> = HashMap::new();
        let mut num_leds: HashMap<String, u32> = HashMap::new();
        let mut fixture_chunks: HashMap<String, Vec<(FixtureId, u32)>> = HashMap::new();

        let fixtures = self.collection.fixtures.write();
        let installations = self.collection.installations.write();
        let groups = self.collection.groups.write();

        for (_fixture_id, fixture) in fixtures.iter() {
            led_positions.insert(
                fixture.id().clone().unprotect(),
                fixture.get_all_led_positions(),
            );
            num_leds.insert(fixture.id().clone().unprotect(), fixture.led_count());
            fixture_chunks.insert(
                fixture.id().clone().unprotect(),
                vec![(fixture.id().clone(), fixture.led_count())],
            );
        }

        for (_installation_id, installation) in installations.iter() {
            led_positions.insert(
                installation.id().clone().unprotect(),
                installation.get_all_led_positions(self),
            );
            num_leds.insert(
                installation.id().clone().unprotect(),
                installation.led_count(self),
            );
            fixture_chunks.insert(
                installation.id().clone().unprotect(),
                installation.get_fixture_chunks(self),
            );
        }

        for (_group_id, group) in groups.iter() {
            led_positions.insert(group.id().unprotect(), group.get_all_led_positions(self));
            num_leds.insert(group.id().unprotect(), group.led_count(self));
            fixture_chunks.insert(
                group.id().clone().unprotect(),
                group.get_fixture_chunks(self),
            );
        }

        let context: WorldContextState =
            WorldContextState::new(led_positions, num_leds, fixture_chunks);

        self.led_position_cache
            .lock()
            .unwrap()
            .replace(context.clone());

        context
    }
}

impl EventEmitter<WorldContextEvent> for WorldContext {
    fn subscribe(&self) -> Receiver<WorldContextEvent> {
        self.event_emitter.subscribe()
    }
}

impl WorldContextCollection {
    pub fn new() -> Self {
        WorldContextCollection {
            fixtures: Arc::new(RwLock::new(HashMap::new())),
            installations: Arc::new(RwLock::new(HashMap::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl WorldContextState {
    fn new(
        led_positions: HashMap<String, Vec<Coord>>,
        num_leds: HashMap<String, u32>,
        fixture_chunks: HashMap<String, Vec<(FixtureId, u32)>>,
    ) -> Self {
        WorldContextState {
            led_positions,
            num_leds,
            fixture_chunks,
        }
    }
}
