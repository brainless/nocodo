//! Dedicated static checker for generated `nocodo_praxis` spec code.
//!
//! See `internal-docs/MILESTONE_PRAXIS_v1.md` §7 (Static Checker) and §10
//! Phase 5. Replaces the Phase 3.5 `cargo check`-stderr-to-PO wrapper
//! (`crate::checker::run_cargo_check`, kept alongside this module per §11
//! risk 5 — `cargo check` still catches real type errors this checker can't)
//! with a dedicated, structured checker that produces typed [`Gap`]s.
//!
//! No LLM call, no tree-sitter (deferred to "future" per §7.3 point 3) — just
//! regex extraction over the generated Rust source. This is tractable because
//! `record_spec_fact`'s serializers (`agents/src/spec_fact/*.rs`) always emit
//! one exact shape per type; the checker only has to understand that shape,
//! not arbitrary Rust.
//!
//! # Pipeline
//!
//! 1. [`SpecIndex::load_from_project`] parses every declared static
//!    (persona/permission/role/state/entity) out of
//!    `backend/src/praxis/specs/*.rs` into an in-memory index.
//! 2. [`extract_id_refs`] pulls every `RoleId("...")`/`StateId("...")`/etc.
//!    literal out of a chunk of generated code (C2 input).
//! 3. `check_*` functions each run one check category (C1-C4) and return
//!    `Vec<Gap>`.
//! 4. [`map_gaps_to_questions`] turns gaps into plain-language PO questions
//!    (§7.2).
//! 5. [`run_static_checks`] wires 1-4 together against a project root — this
//!    is what the backend orchestration (§8.3) calls on `trigger_codegen`.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use regex::Regex;

// ---------------------------------------------------------------------------
// IdKind / IdRefs — C2 vocabulary
// ---------------------------------------------------------------------------

/// Which `nocodo_praxis` newtype an extracted id literal belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdKind {
    Persona,
    Permission,
    Role,
    State,
    Entity,
}

impl IdKind {
    fn from_constructor(s: &str) -> Option<Self> {
        match s {
            "PersonaId" => Some(IdKind::Persona),
            "PermissionId" => Some(IdKind::Permission),
            "RoleId" => Some(IdKind::Role),
            "StateId" => Some(IdKind::State),
            "EntityId" => Some(IdKind::Entity),
            _ => None,
        }
    }
}

/// One `SomeIdKind("literal")` occurrence found in a chunk of generated code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdRef {
    pub kind: IdKind,
    pub id: String,
}

/// Every praxis id literal found in a chunk of generated code — the raw
/// material for C2 (referential integrity).
#[derive(Debug, Clone, Default)]
pub struct IdRefs {
    pub refs: Vec<IdRef>,
}

impl IdRefs {
    pub fn all(&self) -> impl Iterator<Item = &IdRef> {
        self.refs.iter()
    }
}

/// Regex-extract every `RoleId("...")`, `StateId("...")`, `EntityId("...")`,
/// `PermissionId("...")`, `PersonaId("...")` literal from `code`
/// (MILESTONE_PRAXIS_v1.md §7.1 C2, §7.3 point 2).
///
/// This includes *declaration* sites (e.g. the `PersonaId("admin")` inside
/// `pub const PERSONA_ADMIN: PersonaId = PersonaId("admin");`) as well as
/// *usage* sites (e.g. inside a `Role`'s `personas: &[PersonaId("admin")]`).
/// That's intentional — [`check_referential_integrity`] cross-references
/// every occurrence against [`SpecIndex`], and a declaration site always
/// resolves against its own declaration, so it never produces a false gap.
pub fn extract_id_refs(code: &str) -> IdRefs {
    let re = Regex::new(
        r#"(PersonaId|PermissionId|RoleId|StateId|EntityId)\(\s*"((?:[^"\\]|\\.)*)"\s*\)"#,
    )
    .expect("static regex is valid");

    let refs = re
        .captures_iter(code)
        .filter_map(|c| {
            let kind = IdKind::from_constructor(&c[1])?;
            Some(IdRef {
                kind,
                id: c[2].to_string(),
            })
        })
        .collect();

    IdRefs { refs }
}

// ---------------------------------------------------------------------------
// SpecIndex — declared statics loaded from backend/src/praxis/specs/*.rs
// ---------------------------------------------------------------------------

/// One declared `Role`'s cross-references, as parsed out of `roles.rs`.
#[derive(Debug, Clone, Default)]
struct RoleFactIndex {
    permission_ids: Vec<String>,
    persona_ids: Vec<String>,
}

/// One declared `State`'s outgoing transitions, as parsed out of `states.rs`.
#[derive(Debug, Clone, Default)]
struct StateFactIndex {
    /// Target state ids of every outgoing `Transition`. Empty for
    /// `Transitions::Terminal` states — not a gap, structurally distinct
    /// from "we forgot to add transitions" (mirrors `nocodo_praxis`).
    to_states: Vec<String>,
}

/// One declared `Entity`'s lifecycle, as parsed out of `entities.rs`.
#[derive(Debug, Clone, Default)]
struct EntityFactIndex {
    state_ids: Vec<String>,
}

/// A single declared transition: which entity, from which state, to which
/// state. The unit `check_guard_coverage` (C3) reasons about.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransitionSpec {
    pub entity: String,
    pub from: String,
    pub to: String,
}

/// In-memory index of every declared static in a managed project's
/// `backend/src/praxis/specs/` — personas, permissions, roles, states,
/// entities — plus enough structure (role -> persona/permission ids, state ->
/// outgoing transitions, entity -> state ids) to run C3/C4.
///
/// Regex-based parsing of the generated Rust source (MILESTONE_PRAXIS_v1.md
/// §7.3 point 2) — tractable because `record_spec_fact`'s serializers always
/// emit one exact shape per type (see `agents/src/spec_fact/*.rs`): a
/// `pub const {PREFIX}_{ID}: {Type} = {Type}("id");` declaration immediately
/// followed by a `pub static {PREFIX}_{ID}_DEF: {Type} = {Type} { id:
/// {PREFIX}_{ID}, ... };` (or `{ID}_persona` for personas, which have no
/// body fields worth indexing beyond the id itself). The `id: {CONST},`
/// field inside the static's body is resolved back to its string literal via
/// a `const_name -> id` map built while scanning declarations — the literal
/// itself only ever appears at the declaration site, never inside the
/// static's body.
#[derive(Debug, Clone, Default)]
pub struct SpecIndex {
    personas: HashSet<String>,
    permissions: HashSet<String>,
    roles: HashMap<String, RoleFactIndex>,
    states: HashMap<String, StateFactIndex>,
    entities: HashMap<String, EntityFactIndex>,
    /// `pub const` name -> declared id literal, e.g. `"ROLE_ADMIN" ->
    /// "admin"`. Accumulates across every `ingest` call so cross-file
    /// resolution works even though today's serializers never need it
    /// (const and its owning static are always emitted into the same file).
    const_to_id: HashMap<String, String>,
}

impl SpecIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load the index from `{project_root}/backend/src/praxis/specs/*.rs`.
    /// Missing files (a type never recorded yet) and a missing `specs/`
    /// directory entirely (nothing recorded yet) are not errors — they just
    /// leave the corresponding part of the index empty.
    pub fn load_from_project(project_root: &Path) -> Self {
        let specs_dir = project_root.join("backend/src/praxis/specs");
        let mut index = Self::new();
        for file in [
            "personas.rs",
            "permissions.rs",
            "roles.rs",
            "states.rs",
            "entities.rs",
        ] {
            if let Ok(content) = std::fs::read_to_string(specs_dir.join(file)) {
                index.ingest(&content);
            }
        }
        index
    }

    /// Parse one spec source file's worth of Rust and merge its declared
    /// statics into this index. Safe to call multiple times / on
    /// concatenated sources — declarations are keyed by id, so re-ingesting
    /// the same content is idempotent.
    pub fn ingest(&mut self, source: &str) {
        // Declarations first — the block parsers below depend on
        // `const_to_id` being populated for this source.
        self.ingest_declarations(source);
        self.ingest_roles(source);
        self.ingest_states(source);
        self.ingest_entities(source);
    }

    /// `pub const {CONST}: {Type} = {Type}("id");` — declaration sites for
    /// every kind. Populates the flat `personas`/`permissions` sets, seeds an
    /// (empty, filled-in-later) entry for roles/states/entities, and records
    /// `const_name -> id` for the block parsers.
    fn ingest_declarations(&mut self, source: &str) {
        let re = Regex::new(
            r#"pub const (\w+):\s*\w+\s*=\s*(PersonaId|PermissionId|RoleId|StateId|EntityId)\(\s*"((?:[^"\\]|\\.)*)"\s*\);"#,
        )
        .expect("static regex is valid");

        for c in re.captures_iter(source) {
            let const_name = c[1].to_string();
            let kind = IdKind::from_constructor(&c[2]);
            let id = c[3].to_string();
            self.const_to_id.insert(const_name, id.clone());
            match kind {
                Some(IdKind::Persona) => {
                    self.personas.insert(id);
                }
                Some(IdKind::Permission) => {
                    self.permissions.insert(id);
                }
                Some(IdKind::Role) => {
                    self.roles.entry(id).or_default();
                }
                Some(IdKind::State) => {
                    self.states.entry(id).or_default();
                }
                Some(IdKind::Entity) => {
                    self.entities.entry(id).or_default();
                }
                None => {}
            }
        }
    }

    fn ingest_roles(&mut self, source: &str) {
        let marker = Regex::new(r"pub static \w+:\s*Role\s*=\s*Role\s*\{").expect("valid regex");
        for block in find_balanced_blocks(source, &marker) {
            let Some(role_id) = self.resolve_own_id(&block) else {
                continue;
            };
            let permission_ids = extract_id_list(&block, "permissions", "PermissionId");
            let persona_ids = extract_id_list(&block, "personas", "PersonaId");
            self.roles.insert(
                role_id,
                RoleFactIndex {
                    permission_ids,
                    persona_ids,
                },
            );
        }
    }

    fn ingest_states(&mut self, source: &str) {
        let marker =
            Regex::new(r"pub static \w+:\s*State\s*=\s*State\s*\{").expect("valid regex");
        let to_re = Regex::new(r#"to:\s*StateId\(\s*"((?:[^"\\]|\\.)*)"\s*\)"#).expect("valid");
        for block in find_balanced_blocks(source, &marker) {
            let Some(state_id) = self.resolve_own_id(&block) else {
                continue;
            };
            let terminal = block.contains("Transitions::Terminal");
            let to_states = if terminal {
                Vec::new()
            } else {
                to_re
                    .captures_iter(&block)
                    .map(|c| c[1].to_string())
                    .collect()
            };
            self.states.insert(state_id, StateFactIndex { to_states });
        }
    }

    fn ingest_entities(&mut self, source: &str) {
        let marker =
            Regex::new(r"pub static \w+:\s*Entity\s*=\s*Entity\s*\{").expect("valid regex");
        for block in find_balanced_blocks(source, &marker) {
            let Some(entity_id) = self.resolve_own_id(&block) else {
                continue;
            };
            let state_ids = extract_id_list(&block, "states", "StateId");
            self.entities
                .insert(entity_id, EntityFactIndex { state_ids });
        }
    }

    /// Resolve a static block's own id: find its `id: {CONST},` field and
    /// look `{CONST}` up in `const_to_id` (populated by
    /// `ingest_declarations`).
    fn resolve_own_id(&self, block: &str) -> Option<String> {
        let id_field_re = Regex::new(r"id:\s*(\w+),").expect("valid regex");
        let const_name = &id_field_re.captures(block)?[1];
        self.const_to_id.get(const_name).cloned()
    }

    /// Does this index declare `id` under `kind`?
    pub fn contains(&self, kind: IdKind, id: &str) -> bool {
        match kind {
            IdKind::Persona => self.personas.contains(id),
            IdKind::Permission => self.permissions.contains(id),
            IdKind::Role => self.roles.contains_key(id),
            IdKind::State => self.states.contains_key(id),
            IdKind::Entity => self.entities.contains_key(id),
        }
    }

    /// Every transition implied by the declared entities/states graph: for
    /// each entity, for each of its states, for each of that state's
    /// outgoing transitions. Used by [`check_guard_coverage`] (C3).
    pub fn declared_transitions(&self) -> Vec<TransitionSpec> {
        let mut out = Vec::new();
        for (entity_id, entity) in &self.entities {
            for state_id in &entity.state_ids {
                let Some(state) = self.states.get(state_id) else {
                    continue;
                };
                for to in &state.to_states {
                    out.push(TransitionSpec {
                        entity: entity_id.clone(),
                        from: state_id.clone(),
                        to: to.clone(),
                    });
                }
            }
        }
        out
    }

    /// Every declared role, paired with its cross-references — the input to
    /// [`check_spec_completeness`] (C4).
    fn roles_with_refs(&self) -> impl Iterator<Item = (&String, &RoleFactIndex)> {
        self.roles.iter()
    }
}

// ---------------------------------------------------------------------------
// Block parsing helpers
// ---------------------------------------------------------------------------

/// Find every occurrence of `marker` in `source` and return the
/// brace-balanced block that follows its match (from just after the opening
/// `{` the marker itself ends on, up to — but excluding — the matching
/// closing `}`).
///
/// `marker`'s pattern must end at an opening `{` (e.g. `...Role\s*\{`).
fn find_balanced_blocks(source: &str, marker: &Regex) -> Vec<String> {
    let mut out = Vec::new();
    for m in marker.find_iter(source) {
        let start = m.end(); // just after the opening '{'
        let mut depth = 1i32;
        let mut end = None;
        for (offset, ch) in source[start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            out.push(source[start..end].to_string());
        }
    }
    out
}

/// Extract every `{ctor}("...")` id literal inside the `{field}: &[...]`
/// slice literal of a block, e.g. `extract_id_list(block, "permissions",
/// "PermissionId")` on a `Role { ..., permissions: &[PermissionId("a"),
/// PermissionId("b")], ... }` block returns `["a", "b"]`.
fn extract_id_list(block: &str, field: &str, ctor: &str) -> Vec<String> {
    let list_re = Regex::new(&format!(r"{field}:\s*&\[([^\]]*)\]")).expect("valid regex");
    let Some(caps) = list_re.captures(block) else {
        return Vec::new();
    };
    let inner = &caps[1];
    let item_re = Regex::new(&format!(r#"{ctor}\(\s*"((?:[^"\\]|\\.)*)"\s*\)"#)).expect("valid");
    item_re
        .captures_iter(inner)
        .map(|c| c[1].to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Gap — the unit every check category produces
// ---------------------------------------------------------------------------

/// A structured gap found by the static checker (MILESTONE_PRAXIS_v1.md §7.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gap {
    /// C1: `Unresolved::Pending { reason: "..." }` or
    /// `TransitionError::Unresolved { reason: "..." }` found in generated
    /// code.
    Unresolved { reason: String },
    /// C2: an id literal was referenced (e.g. `RoleId("admin")`) but no
    /// matching static is declared anywhere in the spec.
    UndeclaredRef { kind: IdKind, id: String },
    /// C3: a `State` declares an outgoing `Transition` but no
    /// `impl TransitionGuard` with matching `ENTITY`/`FROM`/`TO` exists.
    MissingGuard {
        entity: String,
        from: String,
        to: String,
    },
    /// C4: a declared `Role` has no associated personas — "who typically
    /// holds this role?" was never answered.
    RoleMissingPersonas { role_id: String },
    /// C4: a declared `Role` has no associated permissions — "what can this
    /// role actually do?" was never answered.
    RoleMissingPermissions { role_id: String },
}

// ---------------------------------------------------------------------------
// C1: Unresolved nodes
// ---------------------------------------------------------------------------

/// Scan `code` for `Unresolved::Pending { reason: "..." }` and
/// `TransitionError::Unresolved { reason: "..." }` patterns
/// (MILESTONE_PRAXIS_v1.md §7.1 C1).
pub fn check_unresolved(code: &str) -> Vec<Gap> {
    let mut gaps = Vec::new();
    for marker in ["Unresolved::Pending", "TransitionError::Unresolved"] {
        let mut search_from = 0usize;
        while let Some(rel) = code[search_from..].find(marker) {
            let marker_start = search_from + rel;
            let after_marker = &code[marker_start + marker.len()..];
            let Some(brace_rel) = after_marker.find('{') else {
                break;
            };
            let content_start = marker_start + marker.len() + brace_rel + 1;
            let mut depth = 1i32;
            let mut content_end = None;
            for (offset, ch) in code[content_start..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            content_end = Some(content_start + offset);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(content_end) = content_end else {
                break;
            };
            let block = &code[content_start..content_end];
            if let Some(reason) = extract_reason(block) {
                gaps.push(Gap::Unresolved {
                    reason: reason.to_string(),
                });
            }
            search_from = content_end;
        }
    }
    gaps
}

/// Extract the `reason: "..."` value from an `Unresolved`/`TransitionError`
/// block, handling escaped quotes.
fn extract_reason(block: &str) -> Option<&str> {
    let reason_pos = block.find("reason:")?;
    let after = block[reason_pos + "reason:".len()..].trim_start();
    let quote_start = after.find('"')?;
    let after_quote = &after[quote_start + 1..];

    let mut end = 0;
    let mut escaped = false;
    for c in after_quote.chars() {
        if escaped {
            escaped = false;
            end += c.len_utf8();
            continue;
        }
        if c == '\\' {
            escaped = true;
            end += c.len_utf8();
            continue;
        }
        if c == '"' {
            break;
        }
        end += c.len_utf8();
    }
    Some(&after_quote[..end])
}

// ---------------------------------------------------------------------------
// C2: Referential integrity
// ---------------------------------------------------------------------------

/// Cross-reference every id in `refs` against `spec`; anything referenced but
/// not declared is a gap (MILESTONE_PRAXIS_v1.md §7.1 C2).
///
/// Deduplicates by `(kind, id)` — a `RoleId("admin")` referenced ten times in
/// generated code produces one gap, not ten.
pub fn check_referential_integrity(refs: &IdRefs, spec: &SpecIndex) -> Vec<Gap> {
    let mut seen = HashSet::new();
    let mut gaps = Vec::new();
    for r in refs.all() {
        if spec.contains(r.kind, &r.id) {
            continue;
        }
        if seen.insert((r.kind, r.id.clone())) {
            gaps.push(Gap::UndeclaredRef {
                kind: r.kind,
                id: r.id.clone(),
            });
        }
    }
    gaps
}

// ---------------------------------------------------------------------------
// C3: Guard function coverage
// ---------------------------------------------------------------------------

/// Regex-extract every `impl TransitionGuard<...> for ...` block's
/// `ENTITY`/`FROM`/`TO` consts from `code` (the concatenated contents of
/// `backend/src/praxis/transitions/*.rs`, once Phase 6 exists).
fn implemented_transitions(code: &str) -> HashSet<TransitionSpec> {
    let marker = Regex::new(r"impl\s+TransitionGuard\s*<[^{]*\{").expect("valid regex");
    let entity_re =
        Regex::new(r#"const ENTITY:\s*EntityId\s*=\s*EntityId\(\s*"((?:[^"\\]|\\.)*)"\s*\);"#)
            .expect("valid");
    let from_re =
        Regex::new(r#"const FROM:\s*StateId\s*=\s*StateId\(\s*"((?:[^"\\]|\\.)*)"\s*\);"#)
            .expect("valid");
    let to_re = Regex::new(r#"const TO:\s*StateId\s*=\s*StateId\(\s*"((?:[^"\\]|\\.)*)"\s*\);"#)
        .expect("valid");

    let mut out = HashSet::new();
    for block in find_balanced_blocks(code, &marker) {
        let (Some(entity), Some(from), Some(to)) = (
            entity_re.captures(&block).map(|c| c[1].to_string()),
            from_re.captures(&block).map(|c| c[1].to_string()),
            to_re.captures(&block).map(|c| c[1].to_string()),
        ) else {
            continue;
        };
        out.insert(TransitionSpec { entity, from, to });
    }
    out
}

/// For every declared transition (entity/from/to, derived from `spec`'s
/// entity -> state -> transition graph), verify a matching
/// `impl TransitionGuard` exists in `guard_code` (MILESTONE_PRAXIS_v1.md §7.1
/// C3).
///
/// `guard_code` is the concatenated contents of
/// `backend/src/praxis/transitions/*.rs`. That directory doesn't exist until
/// Phase 6 builds the `state_transition` RustEngineer mode — pass an empty
/// string (or the result of a missing-directory read) in that case; every
/// declared transition will correctly surface as a "not implemented" gap
/// rather than erroring.
pub fn check_guard_coverage(spec: &SpecIndex, guard_code: &str) -> Vec<Gap> {
    let implemented = implemented_transitions(guard_code);
    spec.declared_transitions()
        .into_iter()
        .filter(|t| !implemented.contains(t))
        .map(|t| Gap::MissingGuard {
            entity: t.entity,
            from: t.from,
            to: t.to,
        })
        .collect()
}

/// Read and concatenate every `.rs` file directly under
/// `{project_root}/backend/src/praxis/transitions/`. Returns an empty string
/// if the directory doesn't exist yet (Phase 6 hasn't run) — not an error.
pub fn load_transition_guard_code(project_root: &Path) -> String {
    let dir = project_root.join("backend/src/praxis/transitions");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return String::new();
    };
    let mut out = String::new();
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                out.push_str(&content);
                out.push('\n');
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// C4: Missing spec declarations (spec completeness)
// ---------------------------------------------------------------------------

/// For every declared `Role`, flag it if it has no associated personas
/// and/or no associated permissions (MILESTONE_PRAXIS_v1.md §7.1 C4: "a Role
/// with permissions but no personas is suspect", and symmetrically for
/// permissions).
///
/// This is deliberately distinct from [`check_referential_integrity`]: C2
/// catches "referenced an id that isn't declared"; this catches "declared a
/// role but never said who holds it / what it can do" — an empty list, not a
/// dangling reference.
pub fn check_spec_completeness(spec: &SpecIndex) -> Vec<Gap> {
    let mut gaps = Vec::new();
    for (role_id, role) in spec.roles_with_refs() {
        if role.persona_ids.is_empty() {
            gaps.push(Gap::RoleMissingPersonas {
                role_id: role_id.clone(),
            });
        }
        if role.permission_ids.is_empty() {
            gaps.push(Gap::RoleMissingPermissions {
                role_id: role_id.clone(),
            });
        }
    }
    gaps
}

// ---------------------------------------------------------------------------
// §7.2: Gap -> PO question mapping
// ---------------------------------------------------------------------------

/// A gap translated into a plain-language question PO can ask the user
/// (MILESTONE_PRAXIS_v1.md §7.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoQuestion {
    /// The gap this question addresses — kept so the backend/PO can dedupe
    /// or correlate a user's answer back to the originating gap.
    pub gap: Gap,
    /// Plain-language question text.
    pub question: String,
}

/// Turn each gap into a structured question PO could ask the user
/// (MILESTONE_PRAXIS_v1.md §7.2's table).
pub fn map_gaps_to_questions(gaps: &[Gap]) -> Vec<PoQuestion> {
    gaps.iter()
        .map(|gap| {
            let question = match gap {
                Gap::Unresolved { reason } => {
                    format!("One thing is still unclear: {reason} — can you clarify?")
                }
                Gap::UndeclaredRef {
                    kind: IdKind::Role,
                    id,
                } => format!("You mentioned '{id}'. Can you describe this role?"),
                Gap::UndeclaredRef {
                    kind: IdKind::Persona,
                    id,
                } => format!("Can you describe the typical '{id}'?"),
                Gap::UndeclaredRef {
                    kind: IdKind::Permission,
                    id,
                } => format!("What exactly does '{id}' mean?"),
                Gap::UndeclaredRef {
                    kind: IdKind::State,
                    id,
                } => format!(
                    "You mentioned a '{id}' state — what does it mean, and when does something enter it?"
                ),
                Gap::UndeclaredRef {
                    kind: IdKind::Entity,
                    id,
                } => format!(
                    "What is a '{id}' in your system? What information does it carry?"
                ),
                Gap::MissingGuard { entity, from, to } => format!(
                    "Who can move a {entity} from '{from}' to '{to}', and under what conditions?"
                ),
                Gap::RoleMissingPersonas { role_id } => format!(
                    "Who typically holds the '{role_id}' role? Can you describe them?"
                ),
                Gap::RoleMissingPermissions { role_id } => format!(
                    "What should someone with the '{role_id}' role actually be able to do?"
                ),
            };
            PoQuestion {
                gap: gap.clone(),
                question,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Top-level orchestration
// ---------------------------------------------------------------------------

/// Run every check category (C1-C4) against a managed project and return the
/// combined, deduplicated gap list — what the backend orchestration (§8.3)
/// calls on `trigger_codegen`, replacing/augmenting the Phase 3.5
/// `run_cargo_check` wrapper.
///
/// Reads `backend/src/praxis/specs/*.rs` (via [`SpecIndex::load_from_project`])
/// and `backend/src/praxis/transitions/*.rs` (via
/// [`load_transition_guard_code`], empty until Phase 6). Pure filesystem
/// reads — no LLM call, no `cargo` subprocess.
pub fn run_static_checks(project_root: &Path) -> Vec<Gap> {
    let spec = SpecIndex::load_from_project(project_root);
    let specs_text = concatenated_spec_source(project_root);
    let guard_code = load_transition_guard_code(project_root);

    let mut gaps = Vec::new();
    gaps.extend(check_unresolved(&specs_text));
    let refs = extract_id_refs(&specs_text);
    gaps.extend(check_referential_integrity(&refs, &spec));
    gaps.extend(check_guard_coverage(&spec, &guard_code));
    gaps.extend(check_spec_completeness(&spec));
    gaps
}

fn concatenated_spec_source(project_root: &Path) -> String {
    let specs_dir = project_root.join("backend/src/praxis/specs");
    let mut out = String::new();
    for file in [
        "personas.rs",
        "permissions.rs",
        "roles.rs",
        "states.rs",
        "entities.rs",
    ] {
        if let Ok(content) = std::fs::read_to_string(specs_dir.join(file)) {
            out.push_str(&content);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------
    // extract_id_refs
    // -----------------------------------------------------------------

    #[test]
    fn extract_id_refs_finds_every_kind() {
        let code = r#"
            pub const ROLE_ADMIN: RoleId = RoleId("admin");
            pub const STATE_TODO: StateId = StateId("todo");
            pub const ENTITY_TASK: EntityId = EntityId("task");
            pub const PERM_CREATE: PermissionId = PermissionId("create_task");
            pub const PERSONA_ADMIN: PersonaId = PersonaId("admin");
        "#;
        let refs = extract_id_refs(code);
        assert_eq!(refs.refs.len(), 5);
        assert!(refs
            .all()
            .any(|r| r.kind == IdKind::Role && r.id == "admin"));
        assert!(refs
            .all()
            .any(|r| r.kind == IdKind::State && r.id == "todo"));
        assert!(refs
            .all()
            .any(|r| r.kind == IdKind::Entity && r.id == "task"));
        assert!(refs
            .all()
            .any(|r| r.kind == IdKind::Permission && r.id == "create_task"));
        assert!(refs
            .all()
            .any(|r| r.kind == IdKind::Persona && r.id == "admin"));
    }

    #[test]
    fn extract_id_refs_empty_on_no_matches() {
        let refs = extract_id_refs("fn main() {}");
        assert!(refs.refs.is_empty());
    }

    // -----------------------------------------------------------------
    // SpecIndex — parsing fixtures (hand-written, matching the exact shape
    // `agents/src/spec_fact/*.rs` serializers emit)
    // -----------------------------------------------------------------

    const PERSONAS_RS: &str = r#"
use nocodo_praxis::auth::{PersonaId, UserPersona};
use nocodo_praxis::primitives::AtLeastOne;
use nocodo_praxis::provenance::Provenance;

const CONV_SESSION_1: Provenance = Provenance::Conversation {
    id: "session-1",
    excerpt: "Admins manage the team",
};

pub const PERSONA_ADMIN: PersonaId = PersonaId("admin");

#[allow(non_upper_case_globals)]
pub static ADMIN_persona: UserPersona = UserPersona {
    id: PERSONA_ADMIN,
    name: "Team Admin",
    description: "Manages team and task assignment",
    goals: &["Manage team membership"],
    pain_points: &["Manual tracking"],
    provenance: AtLeastOne { head: CONV_SESSION_1, tail: &[] },
};
"#;

    const PERMISSIONS_RS: &str = r#"
use nocodo_praxis::auth::{PermissionId, Permission};
use nocodo_praxis::primitives::AtLeastOne;
use nocodo_praxis::provenance::Provenance;

const CONV_SESSION_1: Provenance = Provenance::Conversation {
    id: "session-1",
    excerpt: "Create a new task",
};

pub const PERM_CREATE_TASK: PermissionId = PermissionId("create_task");

pub static PERM_CREATE_TASK_DEF: Permission = Permission {
    id: PERM_CREATE_TASK,
    description: "Create a new task",
    provenance: AtLeastOne { head: CONV_SESSION_1, tail: &[] },
};
"#;

    /// A role referencing the admin persona and create_task permission —
    /// both declared above. No gaps expected against `PERSONAS_RS` +
    /// `PERMISSIONS_RS`.
    const ROLES_RS_COMPLETE: &str = r#"
use nocodo_praxis::auth::{RoleId, Role, RoleSemantics, PermissionId, PersonaId};
use nocodo_praxis::primitives::AtLeastOne;
use nocodo_praxis::provenance::Provenance;

const CONV_SESSION_1: Provenance = Provenance::Conversation {
    id: "session-1",
    excerpt: "Team admin",
};

pub const ROLE_ADMIN: RoleId = RoleId("admin");

pub static ROLE_ADMIN_DEF: Role = Role {
    id: ROLE_ADMIN,
    description: "Team admin",
    semantics: RoleSemantics::Flat,
    permissions: &[PermissionId("create_task")],
    personas: &[PersonaId("admin")],
    provenance: AtLeastOne { head: CONV_SESSION_1, tail: &[] },
};
"#;

    /// A second role in the same file, with no personas/permissions and
    /// referencing an undeclared persona — used to exercise both C2 and C4.
    const ROLES_RS_WITH_GAPS: &str = r#"
use nocodo_praxis::auth::{RoleId, Role, RoleSemantics, PermissionId, PersonaId};
use nocodo_praxis::primitives::AtLeastOne;
use nocodo_praxis::provenance::Provenance;

const CONV_SESSION_2: Provenance = Provenance::Conversation {
    id: "session-2",
    excerpt: "Reviewer",
};

pub const ROLE_REVIEWER: RoleId = RoleId("reviewer");

pub static ROLE_REVIEWER_DEF: Role = Role {
    id: ROLE_REVIEWER,
    description: "Reviews submissions",
    semantics: RoleSemantics::Flat,
    permissions: &[],
    personas: &[PersonaId("reviewer_persona")],
    provenance: AtLeastOne { head: CONV_SESSION_2, tail: &[] },
};
"#;

    const STATES_RS: &str = r#"
use nocodo_praxis::auth::{RoleId, PermissionId};
use nocodo_praxis::primitives::AtLeastOne;
use nocodo_praxis::provenance::Provenance;
use nocodo_praxis::statemachine::{State, StateId, Transition, Transitions, TransitionCondition};

const CONV_SESSION_1: Provenance = Provenance::Conversation {
    id: "session-1",
    excerpt: "Task lifecycle",
};

pub const STATE_TODO: StateId = StateId("todo");

pub static STATE_TODO_DEF: State = State {
    id: STATE_TODO,
    description: "Not started",
    transitions: Transitions::To(AtLeastOne {
        head: Transition {
            to: StateId("in_progress"),
            permitted_roles: AtLeastOne { head: RoleId("admin"), tail: &[] },
            condition: TransitionCondition::Always,
            provenance: &[CONV_SESSION_1],
        },
        tail: &[],
    }),
    provenance: &[CONV_SESSION_1],
};

pub const STATE_IN_PROGRESS: StateId = StateId("in_progress");

pub static STATE_IN_PROGRESS_DEF: State = State {
    id: STATE_IN_PROGRESS,
    description: "Being worked on",
    transitions: Transitions::Terminal,
    provenance: &[CONV_SESSION_1],
};
"#;

    const ENTITIES_RS: &str = r#"
use nocodo_praxis::entity::{Entity, EntityId, Field};
use nocodo_praxis::primitives::{AtLeastOne, Unresolved};
use nocodo_praxis::provenance::Provenance;
use nocodo_praxis::statemachine::StateId;

const CONV_SESSION_1: Provenance = Provenance::Conversation {
    id: "session-1",
    excerpt: "A unit of work",
};

pub const ENTITY_TASK: EntityId = EntityId("task");

pub static ENTITY_TASK_DEF: Entity = Entity {
    id: ENTITY_TASK,
    description: "A unit of work",
    fields: &[],
    states: &[StateId("todo"), StateId("in_progress")],
    invariants: &[],
    provenance: &[CONV_SESSION_1],
};
"#;

    #[test]
    fn spec_index_loads_declared_ids_of_every_kind() {
        let mut idx = SpecIndex::new();
        idx.ingest(PERSONAS_RS);
        idx.ingest(PERMISSIONS_RS);
        idx.ingest(ROLES_RS_COMPLETE);
        idx.ingest(STATES_RS);
        idx.ingest(ENTITIES_RS);

        assert!(idx.contains(IdKind::Persona, "admin"));
        assert!(idx.contains(IdKind::Permission, "create_task"));
        assert!(idx.contains(IdKind::Role, "admin"));
        assert!(idx.contains(IdKind::State, "todo"));
        assert!(idx.contains(IdKind::State, "in_progress"));
        assert!(idx.contains(IdKind::Entity, "task"));
        assert!(!idx.contains(IdKind::Role, "nonexistent"));
    }

    #[test]
    fn spec_index_declared_transitions_from_entity_state_graph() {
        let mut idx = SpecIndex::new();
        idx.ingest(STATES_RS);
        idx.ingest(ENTITIES_RS);

        let transitions = idx.declared_transitions();
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].entity, "task");
        assert_eq!(transitions[0].from, "todo");
        assert_eq!(transitions[0].to, "in_progress");
    }

    // -----------------------------------------------------------------
    // C1: check_unresolved
    // -----------------------------------------------------------------

    #[test]
    fn check_unresolved_finds_pending_invariant() {
        let code = r#"
            invariants: &[Unresolved::Pending {
                reason: "Constraints for 'title' not specified",
                provenance: AtLeastOne { head: Provenance::Inferred { reason: "x", from: &[] }, tail: &[] },
            }],
        "#;
        let gaps = check_unresolved(code);
        assert_eq!(gaps.len(), 1);
        assert_eq!(
            gaps[0],
            Gap::Unresolved {
                reason: "Constraints for 'title' not specified".to_string()
            }
        );
    }

    #[test]
    fn check_unresolved_finds_transition_error_unresolved() {
        let code = r#"
            return Err(TransitionError::Unresolved {
                reason: "PRD does not specify who can cancel a task",
            });
        "#;
        let gaps = check_unresolved(code);
        assert_eq!(gaps.len(), 1);
        assert_eq!(
            gaps[0],
            Gap::Unresolved {
                reason: "PRD does not specify who can cancel a task".to_string()
            }
        );
    }

    #[test]
    fn check_unresolved_empty_on_clean_code() {
        let gaps = check_unresolved(ENTITIES_RS);
        // ENTITIES_RS above has no fields, so no Unresolved::Pending appears.
        assert!(gaps.is_empty());
    }

    #[test]
    fn check_unresolved_finds_multiple() {
        let code = r#"
            Unresolved::Pending { reason: "first gap", provenance: X },
            Unresolved::Pending { reason: "second gap", provenance: Y },
        "#;
        let gaps = check_unresolved(code);
        assert_eq!(gaps.len(), 2);
    }

    // -----------------------------------------------------------------
    // C2: check_referential_integrity
    // -----------------------------------------------------------------

    #[test]
    fn referential_integrity_passes_when_all_refs_declared() {
        let mut spec = SpecIndex::new();
        spec.ingest(PERSONAS_RS);
        spec.ingest(PERMISSIONS_RS);
        spec.ingest(ROLES_RS_COMPLETE);

        let refs = extract_id_refs(ROLES_RS_COMPLETE);
        let gaps = check_referential_integrity(&refs, &spec);
        assert!(gaps.is_empty(), "unexpected gaps: {gaps:?}");
    }

    #[test]
    fn referential_integrity_flags_undeclared_persona_ref() {
        let mut spec = SpecIndex::new();
        // Note: reviewer_persona is never declared in personas.rs.
        spec.ingest(ROLES_RS_WITH_GAPS);

        let refs = extract_id_refs(ROLES_RS_WITH_GAPS);
        let gaps = check_referential_integrity(&refs, &spec);
        assert!(gaps.contains(&Gap::UndeclaredRef {
            kind: IdKind::Persona,
            id: "reviewer_persona".to_string(),
        }));
    }

    #[test]
    fn referential_integrity_dedupes_repeated_refs() {
        let code = r#"RoleId("admin") RoleId("admin") RoleId("admin")"#;
        let refs = extract_id_refs(code);
        let spec = SpecIndex::new(); // nothing declared
        let gaps = check_referential_integrity(&refs, &spec);
        assert_eq!(gaps.len(), 1);
    }

    // -----------------------------------------------------------------
    // C3: check_guard_coverage
    // -----------------------------------------------------------------

    #[test]
    fn guard_coverage_flags_declared_transition_with_no_guard_dir() {
        let mut spec = SpecIndex::new();
        spec.ingest(STATES_RS);
        spec.ingest(ENTITIES_RS);

        // No transitions/ directory yet (Phase 6 not built) — empty guard code.
        let gaps = check_guard_coverage(&spec, "");
        assert_eq!(gaps.len(), 1);
        assert_eq!(
            gaps[0],
            Gap::MissingGuard {
                entity: "task".to_string(),
                from: "todo".to_string(),
                to: "in_progress".to_string(),
            }
        );
    }

    #[test]
    fn guard_coverage_passes_when_matching_guard_impl_exists() {
        let mut spec = SpecIndex::new();
        spec.ingest(STATES_RS);
        spec.ingest(ENTITIES_RS);

        let guard_code = r#"
            struct StartTask;
            impl TransitionGuard<(User, Task), Task> for StartTask {
                const ENTITY: EntityId = EntityId("task");
                const FROM: StateId = StateId("todo");
                const TO: StateId = StateId("in_progress");
                fn guard(input: (User, Task)) -> Result<Task, TransitionError> {
                    Ok(input.1)
                }
            }
        "#;
        let gaps = check_guard_coverage(&spec, guard_code);
        assert!(gaps.is_empty(), "unexpected gaps: {gaps:?}");
    }

    #[test]
    fn guard_coverage_no_gaps_when_no_transitions_declared() {
        let spec = SpecIndex::new(); // nothing declared at all
        let gaps = check_guard_coverage(&spec, "");
        assert!(gaps.is_empty());
    }

    // -----------------------------------------------------------------
    // C4: check_spec_completeness
    // -----------------------------------------------------------------

    #[test]
    fn spec_completeness_passes_for_role_with_personas_and_permissions() {
        let mut spec = SpecIndex::new();
        spec.ingest(ROLES_RS_COMPLETE);
        let gaps = check_spec_completeness(&spec);
        assert!(gaps.is_empty(), "unexpected gaps: {gaps:?}");
    }

    #[test]
    fn spec_completeness_flags_role_with_no_personas() {
        let mut spec = SpecIndex::new();
        spec.ingest(ROLES_RS_WITH_GAPS);
        let gaps = check_spec_completeness(&spec);
        assert!(gaps.contains(&Gap::RoleMissingPermissions {
            role_id: "reviewer".to_string()
        }));
        // reviewer *does* list a persona (even though it's undeclared —
        // that's a C2 concern, not C4), so no RoleMissingPersonas gap.
        assert!(!gaps.contains(&Gap::RoleMissingPersonas {
            role_id: "reviewer".to_string()
        }));
    }

    // -----------------------------------------------------------------
    // map_gaps_to_questions
    // -----------------------------------------------------------------

    #[test]
    fn map_gaps_to_questions_produces_one_question_per_gap() {
        let gaps = vec![
            Gap::UndeclaredRef {
                kind: IdKind::Role,
                id: "admin".to_string(),
            },
            Gap::RoleMissingPersonas {
                role_id: "member".to_string(),
            },
        ];
        let questions = map_gaps_to_questions(&gaps);
        assert_eq!(questions.len(), 2);
        assert!(questions[0].question.contains("admin"));
        assert!(questions[1].question.contains("member"));
    }

    #[test]
    fn map_gaps_to_questions_empty_on_no_gaps() {
        assert!(map_gaps_to_questions(&[]).is_empty());
    }

    // -----------------------------------------------------------------
    // Integration: a small hand-built scenario exercising the full pipeline
    // via a real project_root (MILESTONE_PRAXIS_v1.md §10 Phase 5 point 9).
    // -----------------------------------------------------------------

    fn scratch_project_root() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "nocodo-agents-static-checker-test-{}-{}",
            std::process::id(),
            n
        ));
        std::fs::create_dir_all(dir.join("backend/src/praxis/specs")).unwrap();
        dir
    }

    #[test]
    fn run_static_checks_end_to_end_scenario() {
        let root = scratch_project_root();
        let specs_dir = root.join("backend/src/praxis/specs");

        // One persona (admin), one role referencing it (ROLE_ADMIN — no
        // gaps), and one unreferenced/incomplete role (ROLE_REVIEWER — no
        // permissions declared, and its persona ref is undeclared).
        std::fs::write(specs_dir.join("personas.rs"), PERSONAS_RS).unwrap();
        std::fs::write(specs_dir.join("permissions.rs"), PERMISSIONS_RS).unwrap();
        let roles_combined = format!("{ROLES_RS_COMPLETE}\n{ROLES_RS_WITH_GAPS}");
        std::fs::write(specs_dir.join("roles.rs"), &roles_combined).unwrap();

        let gaps = run_static_checks(&root);

        // Expected gap: reviewer_persona referenced but not declared (C2).
        assert!(
            gaps.contains(&Gap::UndeclaredRef {
                kind: IdKind::Persona,
                id: "reviewer_persona".to_string(),
            }),
            "expected undeclared persona gap, got: {gaps:?}"
        );
        // Expected gap: ROLE_REVIEWER has no permissions (C4).
        assert!(
            gaps.contains(&Gap::RoleMissingPermissions {
                role_id: "reviewer".to_string(),
            }),
            "expected missing-permissions gap, got: {gaps:?}"
        );
        // Non-gap: ROLE_ADMIN references PERSONA_ADMIN and
        // PERM_CREATE_TASK, both declared — no undeclared-ref gap for either.
        assert!(!gaps.iter().any(|g| matches!(
            g,
            Gap::UndeclaredRef { kind: IdKind::Persona, id } if id == "admin"
        )));
        assert!(!gaps.iter().any(|g| matches!(
            g,
            Gap::UndeclaredRef { kind: IdKind::Permission, id } if id == "create_task"
        )));
        // Non-gap: ROLE_ADMIN has both personas and permissions declared —
        // no completeness gap for it.
        assert!(!gaps.contains(&Gap::RoleMissingPersonas {
            role_id: "admin".to_string()
        }));
        assert!(!gaps.contains(&Gap::RoleMissingPermissions {
            role_id: "admin".to_string()
        }));
    }

    #[test]
    fn run_static_checks_zero_gaps_on_complete_spec() {
        let root = scratch_project_root();
        let specs_dir = root.join("backend/src/praxis/specs");
        std::fs::write(specs_dir.join("personas.rs"), PERSONAS_RS).unwrap();
        std::fs::write(specs_dir.join("permissions.rs"), PERMISSIONS_RS).unwrap();
        std::fs::write(specs_dir.join("roles.rs"), ROLES_RS_COMPLETE).unwrap();

        let gaps = run_static_checks(&root);
        assert!(gaps.is_empty(), "expected zero gaps, got: {gaps:?}");
    }

    #[test]
    fn run_static_checks_empty_project_no_specs_dir_yet() {
        // No specs/ directory at all (before any record_spec_fact call).
        let root = std::env::temp_dir().join(format!(
            "nocodo-agents-static-checker-empty-{}",
            std::process::id()
        ));
        let gaps = run_static_checks(&root);
        assert!(gaps.is_empty());
    }

    #[test]
    fn load_transition_guard_code_empty_when_directory_missing() {
        let root = scratch_project_root();
        assert_eq!(load_transition_guard_code(&root), "");
    }

}
