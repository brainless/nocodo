use super::primitives::AtLeastOne;
use super::provenance::Provenance;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoleId(pub &'static str);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PermissionId(pub &'static str);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PersonaId(pub &'static str);

pub enum RoleSemantics {
    Flat,
    Inherits { parent: RoleId },
    Union { of: &'static [RoleId] },
}

pub struct Role {
    pub id: RoleId,
    pub description: &'static str,
    pub semantics: RoleSemantics,
    pub permissions: &'static [PermissionId],
    pub personas: &'static [PersonaId],
    pub provenance: AtLeastOne<Provenance>,
}

pub struct Permission {
    pub id: PermissionId,
    pub description: &'static str,
    pub provenance: AtLeastOne<Provenance>,
}

pub struct UserPersona {
    pub id: PersonaId,
    pub name: &'static str,
    pub description: &'static str,
    pub goals: &'static [&'static str],
    pub pain_points: &'static [&'static str],
    pub provenance: AtLeastOne<Provenance>,
}

pub enum ImplicitRole {
    AnyAuthenticated,
    AnyUser,
}
