//! Where every requirement came from.
//!
//! Every node in the spec graph carries at least one [`Provenance`]. Multiple
//! provenances are allowed — a requirement may be stated in a conversation and
//! clarified in a follow-up.
//!
//! [`PrdValue<T>`] pairs a value with its provenance, carrying origin information
//! into the runtime. The running application can answer "why does this config
//! value exist?"

use super::primitives::AtLeastOne;

#[derive(Debug, Clone)]
/// Where did this requirement come from?
///
/// Every value in the spec carries at least one `Provenance`. The [`Inferred`]
/// variant is critical: it marks LLM assumptions so they are visible and
/// independently auditable.
///
/// [`Inferred`]: Provenance::Inferred
///
/// # Examples
///
/// ```
/// use nocodo_praxis::provenance::Provenance;
///
/// // A requirement explicitly stated in conversation
/// let conv = Provenance::Conversation {
///     id: "session-42",
///     excerpt: "Admins must be able to invite new members.",
/// };
/// assert_eq!(conv.excerpt(), "Admins must be able to invite new members.");
///
/// // An assumption the LLM made — marked for human review
/// let inferred = Provenance::Inferred {
///     reason: "Team coordination requires shared visibility",
///     from: &["session-42"],
/// };
/// assert_eq!(inferred.excerpt(), "Team coordination requires shared visibility");
/// ```
pub enum Provenance {
    /// A requirement extracted from a user-agent conversation.
    /// `id` is a session or message identifier; `excerpt` is verbatim text.
    Conversation {
        id: &'static str,
        excerpt: &'static str,
    },
    /// The LLM inferred this from context — it was **not** explicitly stated
    /// in the PRD. This variant makes inference visible and independently
    /// checkable. Inferred nodes are weaker than stated nodes and should be
    /// prioritised for clarification.
    Inferred {
        reason: &'static str,
        /// IDs of the source nodes that led to this inference.
        from: &'static [&'static str],
    },
    /// A requirement traced to a source file.
    File {
        path: &'static str,
        lines: (u32, u32),
        excerpt: &'static str,
    },
}

impl Provenance {
    /// Returns the human-readable excerpt from any variant.
    pub fn excerpt(&self) -> &'static str {
        match self {
            Provenance::Conversation { excerpt, .. } => excerpt,
            Provenance::Inferred { reason, .. } => reason,
            Provenance::File { excerpt, .. } => excerpt,
        }
    }
}

/// A value that carries its PRD origin into the runtime.
///
/// The running application can answer "why does this value exist?" by inspecting
/// the `provenance` field. Used for configuration values, role definitions, and
/// any artifact that must remain traceable to its source.
///
/// # Examples
///
/// ```
/// use nocodo_praxis::primitives::AtLeastOne;
/// use nocodo_praxis::provenance::{PrdValue, Provenance};
///
/// let prov = AtLeastOne {
///     head: Provenance::Conversation {
///         id: "session-1",
///         excerpt: "Task titles must be at most 200 characters.",
///     },
///     tail: &[],
/// };
///
/// let max_title_length = PrdValue::new(200u32, prov);
/// assert_eq!(max_title_length.value, 200);
/// ```
pub struct PrdValue<T> {
    /// The actual value.
    pub value: T,
    /// Where this value came from — at least one provenance is required.
    pub provenance: AtLeastOne<Provenance>,
}

impl<T> PrdValue<T> {
    /// Construct a `PrdValue` from a value and its provenance.
    pub fn new(value: T, provenance: AtLeastOne<Provenance>) -> Self {
        Self { value, provenance }
    }
}
