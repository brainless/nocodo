//! Identity and access control.
//!
//! Types for modeling who can do what. Roles group permissions together;
//! permissions represent individual actions; user personas describe the humans
//! who hold those roles.
//!
//! [`RoleSemantics`] controls how effective permissions are computed:
//!
//! - [`Flat`](RoleSemantics::Flat) — only the permissions explicitly listed.
//! - [`Inherits`](RoleSemantics::Inherits) — own permissions plus the parent's
//!   effective permissions, resolved recursively.
//! - [`Union`](RoleSemantics::Union) — effective permissions are the union of
//!   all listed roles' permissions.
//!
//! # Examples
//!
//! Constructing personas, permissions, and roles:
//!
//! ```
//! use nocodo_praxis::auth::{
//!     PersonaId, PermissionId, RoleId, RoleSemantics,
//!     UserPersona, Permission, Role,
//! };
//! use nocodo_praxis::primitives::AtLeastOne;
//! use nocodo_praxis::provenance::Provenance;
//!
//! let conv = Provenance::Conversation {
//!     id: "prd-init",
//!     excerpt: "A simple Todo app. Members self-assign tasks. Admins manage the team.",
//! };
//! let prov = AtLeastOne { head: conv, tail: &[] };
//!
//! // ── Personas ──
//!
//! let member_persona = UserPersona {
//!     id: PersonaId("member"),
//!     name: "Team Member",
//!     description: "Anyone who has registered and confirmed their account",
//!     goals: &["Track my own tasks", "See what my team is working on"],
//!     pain_points: &["No visibility into team progress"],
//!     provenance: prov.clone(),
//! };
//!
//! // ── Permissions ──
//!
//! let perm_create_task = Permission {
//!     id: PermissionId("create_task"),
//!     description: "Create a new task",
//!     provenance: prov.clone(),
//! };
//!
//! // ── Roles (Flat) ──
//!
//! static AUTH_PERMS: [PermissionId; 1] = [PermissionId("view_all_tasks")];
//! let all_auth = Role {
//!     id: RoleId("all_authenticated"),
//!     description: "Any user with a valid session",
//!     semantics: RoleSemantics::Flat,
//!     permissions: &AUTH_PERMS,
//!     personas: &[],
//!     provenance: prov.clone(),
//! };
//!
//! // ── Roles (Inherits) ──
//!
//! static MEMBER_PERMS: [PermissionId; 1] = [PermissionId("create_task")];
//! static MEMBER_PERSONAS: [PersonaId; 1] = [PersonaId("member")];
//!
//! let member = Role {
//!     id: RoleId("member"),
//!     description: "Registered team member",
//!     semantics: RoleSemantics::Inherits { parent: RoleId("all_authenticated") },
//!     permissions: &MEMBER_PERMS,
//!     personas: &MEMBER_PERSONAS,
//!     provenance: prov.clone(),
//! };
//!
//! // ── Roles (Union) ──
//!
//! static UNION_OF: [RoleId; 2] = [RoleId("member"), RoleId("reviewer")];
//! let moderator = Role {
//!     id: RoleId("moderator"),
//!     description: "Combines member and reviewer privileges",
//!     semantics: RoleSemantics::Union {
//!         of: &UNION_OF,
//!     },
//!     permissions: &[],
//!     personas: &[],
//!     provenance: prov,
//! };
//! ```

use super::primitives::AtLeastOne;
use super::provenance::Provenance;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A unique identifier for a role (e.g. `RoleId("admin")`).
///
/// Newtype over `&'static str` — always declare as a `const` and reference
/// by the constant, never by raw string literal.
pub struct RoleId(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A unique identifier for a permission (e.g. `PermissionId("create_task")`).
pub struct PermissionId(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A unique identifier for a user persona (e.g. `PersonaId("admin")`).
pub struct PersonaId(pub &'static str);

/// How a role's effective permissions are computed.
///
/// The spec expresses intent; the auth library (Casbin, oso, or custom resolver)
/// implements the resolution. The spec does not hardcode the mechanism.
pub enum RoleSemantics {
    /// Permissions are exactly what is listed on the role. Nothing inherited.
    Flat,
    /// Effective permissions = own permissions + parent's effective permissions.
    /// Resolved recursively at runtime by the auth layer.
    Inherits {
        /// The parent role to inherit permissions from.
        parent: RoleId,
    },
    /// Effective permissions = union of all listed roles' effective permissions.
    Union {
        /// The roles whose permissions are combined.
        of: &'static [RoleId],
    },
}

/// A role that groups permissions and is held by user personas.
///
/// The `permissions` field lists **own** permissions only. Inherited permissions
/// (via [`RoleSemantics::Inherits`] or [`RoleSemantics::Union`]) are **not**
/// listed here — they are resolved at runtime.
pub struct Role {
    /// Unique identifier for this role.
    pub id: RoleId,
    /// Human-readable description.
    pub description: &'static str,
    /// How this role's effective permissions are computed.
    pub semantics: RoleSemantics,
    /// Own permissions only. Inherited permissions are not listed here.
    pub permissions: &'static [PermissionId],
    /// Which user personas typically hold this role.
    pub personas: &'static [PersonaId],
    /// Where this role was defined in the PRD.
    pub provenance: AtLeastOne<Provenance>,
}

/// A single permission representing an action a role may perform.
pub struct Permission {
    /// Unique identifier (e.g. `PermissionId("create_task")`).
    pub id: PermissionId,
    /// Human-readable description of what this permission grants.
    pub description: &'static str,
    /// Where this permission was derived from in the PRD.
    pub provenance: AtLeastOne<Provenance>,
}

/// A user persona — a human role in the system (not a technical role).
///
/// Personas describe who uses the software and what they need. They are the
/// bridge between user research and access control: personas inform which
/// [`Role`]s exist and what [`Permission`]s each role needs.
///
/// # Examples
///
/// ```
/// use nocodo_praxis::auth::{PersonaId, UserPersona};
/// use nocodo_praxis::primitives::AtLeastOne;
/// use nocodo_praxis::provenance::Provenance;
///
/// let conv = Provenance::Conversation {
///     id: "session-1",
///     excerpt: "We need an admin who can manage the team and assign tasks.",
/// };
///
/// let admin = UserPersona {
///     id: PersonaId("admin"),
///     name: "Admin",
///     description: "Elevated user who manages team and task assignment",
///     goals: &["Manage team membership", "Create and assign tasks to anyone"],
///     pain_points: &["Manual task tracking", "No visibility into team progress"],
///     provenance: AtLeastOne { head: conv, tail: &[] },
/// };
///
/// assert_eq!(admin.name, "Admin");
/// assert_eq!(admin.goals.len(), 2);
/// ```
pub struct UserPersona {
    /// Short identifier (e.g. `PersonaId("admin")`).
    pub id: PersonaId,
    /// Human-readable display name (e.g. "Team Admin").
    pub name: &'static str,
    /// One to two sentences describing this persona's role and context.
    pub description: &'static str,
    /// What this persona needs to accomplish with the software.
    pub goals: &'static [&'static str],
    /// What frustrates this persona without the software.
    pub pain_points: &'static [&'static str],
    /// Where this persona was described in the PRD.
    pub provenance: AtLeastOne<Provenance>,
}

/// An explicit role for users that do not have an assigned role.
///
/// "Default" visibility is a hidden assumption. `ImplicitRole` makes it explicit:
/// the absence of an actor is an [`Unresolved`], not a default. Every action has
/// an explicit actor, even if that actor is all authenticated users.
///
/// [`Unresolved`]: super::primitives::Unresolved
pub enum ImplicitRole {
    /// Any user with a valid session, regardless of assigned role.
    AnyAuthenticated,
    /// Any user, including unauthenticated. Use with care.
    AnyUser,
}
