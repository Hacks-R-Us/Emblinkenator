#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct FixtureId {
    #[protected_value]
    id: String
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct InstallationId {
    #[protected_value]
    id: String
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct GroupId {
    #[protected_value]
    id: String
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct AnimationId {
    #[protected_value]
    id: String
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct DeviceId {
    #[protected_value]
    id: String
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, ProtectedId)]
pub struct AuxiliaryId {
    #[protected_value]
    id: String
}
