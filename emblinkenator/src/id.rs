use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct FixtureId {
    #[protected_value]
    id: String,
}

impl Display for FixtureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.unprotect())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct InstallationId {
    #[protected_value]
    id: String,
}

impl Display for InstallationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.unprotect())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct GroupId {
    #[protected_value]
    id: String,
}

impl Display for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.unprotect())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct AnimationId {
    #[protected_value]
    id: String,
}

impl Display for AnimationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.unprotect())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct DeviceId {
    #[protected_value]
    id: String,
}

impl Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.unprotect())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct AuxiliaryId {
    #[protected_value]
    id: String,
}

impl Display for AuxiliaryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.unprotect())
    }
}
