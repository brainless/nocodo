/// Structural building blocks used by all other modules.
///
/// Two primitives underpin the entire crate:
///
/// - [`AtLeastOne<T>`] — a non-empty collection. Encodes "at least one responsible
///   party" as a structural guarantee, not a convention.
/// - [`Unresolved<T>`] — first-class incompleteness. Anything that cannot be
///   encoded in current `nocodo_praxis` types must use `Unresolved` rather than
///   approximate. This blocks downstream codegen explicitly.

#[derive(Debug, Clone)]
/// A collection that must have at least one element.
///
/// The fundamental encoding of "this business rule requires a responsible party."
/// An [`Action`] with no permitted [`Role`], a [`Transition`] with no permitted
/// [`Role`] — these are structurally unrepresentable.
///
/// [`Action`]: super::auth::Role
/// [`Role`]: super::auth::Role
/// [`Transition`]: super::statemachine::Transition
///
/// # Examples
///
/// ```
/// use nocodo_praxis::primitives::AtLeastOne;
///
/// let roles = AtLeastOne {
///     head: "admin",
///     tail: &["member", "viewer"],
/// };
///
/// assert_eq!(roles.len(), 3);
/// assert!(roles.contains(|r| *r == "member"));
/// ```
pub struct AtLeastOne<T: 'static> {
    /// The first element. Always present.
    pub head: T,
    /// Additional elements (may be empty).
    pub tail: &'static [T],
}

impl<T> AtLeastOne<T> {
    /// Returns an iterator over all elements (head first, then tail).
    pub fn all(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.head).chain(self.tail.iter())
    }

    /// Returns `true` if any element satisfies the predicate.
    pub fn contains<F: Fn(&T) -> bool>(&self, f: F) -> bool {
        self.all().any(f)
    }

    /// Returns the total number of elements (at least 1).
    pub fn len(&self) -> usize {
        1 + self.tail.len()
    }
}

#[derive(Debug, Clone)]
/// First-class incompleteness.
///
/// Anything that cannot be encoded in current `nocodo_praxis` types must be
/// wrapped in `Unresolved` rather than approximated. This blocks downstream
/// codegen explicitly and surfaces the gap to the clarification loop.
///
/// # Variants
///
/// - `Resolved(T)` — the value is known, codegen can proceed.
/// - `Pending { reason, provenance }` — not yet decided. Blocks codegen.
/// - `Blocked { by, reason }` — depends on another `Unresolved` being
///   resolved first. Blocks codegen.
///
/// # Examples
///
/// ```
/// use nocodo_praxis::primitives::{AtLeastOne, Unresolved};
/// use nocodo_praxis::provenance::Provenance;
///
/// let conv = Provenance::Conversation { id: "s1", excerpt: "We need a title field." };
///
/// // A resolved invariant
/// let resolved: Unresolved<&str> = Unresolved::Resolved("max 200 chars");
/// assert!(resolved.is_resolved());
/// assert!(!resolved.blocks_codegen());
///
/// // A pending invariant — blocks codegen until answered
/// let pending: Unresolved<&str> = Unresolved::Pending {
///     reason: "Maximum title length not specified",
///     provenance: AtLeastOne { head: conv.clone(), tail: &[] },
/// };
/// assert!(!pending.is_resolved());
/// assert!(pending.blocks_codegen());
/// assert_eq!(pending.reason(), Some("Maximum title length not specified"));
///
/// // A blocked invariant — depends on another gap being resolved first
/// let blocked: Unresolved<&str> = Unresolved::Blocked {
///     by: &["gap-1"],
///     reason: "Cannot define validation until entity fields are decided",
/// };
/// assert!(!blocked.is_resolved());
/// assert_eq!(blocked.reason(), Some("Cannot define validation until entity fields are decided"));
/// ```
pub enum Unresolved<T> {
    /// The value is known — codegen can proceed.
    Resolved(T),
    /// Not yet decided. The `reason` explains what is missing and
    /// `provenance` records where the gap was identified.
    Pending {
        reason: &'static str,
        provenance: AtLeastOne<super::provenance::Provenance>,
    },
    /// Depends on another `Unresolved` being resolved first.
    /// `by` lists the IDs of the blocking gaps.
    Blocked {
        by: &'static [&'static str],
        reason: &'static str,
    },
}

impl<T> Unresolved<T> {
    /// Returns `true` if this is `Resolved`.
    pub fn is_resolved(&self) -> bool {
        matches!(self, Unresolved::Resolved(_))
    }

    /// Returns `true` if codegen should be blocked — i.e., not `Resolved`.
    pub fn blocks_codegen(&self) -> bool {
        !self.is_resolved()
    }

    /// Returns the resolved value, or `None` if still pending or blocked.
    pub fn resolved_value(&self) -> Option<&T> {
        match self {
            Unresolved::Resolved(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the reason for being unresolved, or `None` if `Resolved`.
    pub fn reason(&self) -> Option<&'static str> {
        match self {
            Unresolved::Resolved(_) => None,
            Unresolved::Pending { reason, .. } => Some(reason),
            Unresolved::Blocked { reason, .. } => Some(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_least_one_all_returns_head_and_tail() {
        let a = AtLeastOne {
            head: 1,
            tail: &[2, 3],
        };
        let items: Vec<_> = a.all().copied().collect();
        assert_eq!(items, vec![1, 2, 3]);
    }

    #[test]
    fn at_least_one_len() {
        let a = AtLeastOne {
            head: "a",
            tail: &["b"],
        };
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn at_least_one_contains() {
        let a = AtLeastOne {
            head: 10,
            tail: &[20, 30],
        };
        assert!(a.contains(|x| *x == 20));
        assert!(!a.contains(|x| *x == 99));
    }

    #[test]
    fn at_least_one_head_only() {
        let a = AtLeastOne { head: 42, tail: &[] };
        assert_eq!(a.len(), 1);
        assert!(a.contains(|x| *x == 42));
    }

    #[test]
    fn unresolved_resolved() {
        let u: Unresolved<i32> = Unresolved::Resolved(42);
        assert!(u.is_resolved());
        assert!(!u.blocks_codegen());
        assert_eq!(u.resolved_value(), Some(&42));
        assert_eq!(u.reason(), None);
    }

    #[test]
    fn unresolved_pending() {
        let u: Unresolved<i32> = Unresolved::Pending {
            reason: "not specified",
            provenance: AtLeastOne {
                head: super::super::provenance::Provenance::Conversation {
                    id: "test",
                    excerpt: "test excerpt",
                },
                tail: &[],
            },
        };
        assert!(!u.is_resolved());
        assert!(u.blocks_codegen());
        assert_eq!(u.resolved_value(), None);
        assert_eq!(u.reason(), Some("not specified"));
    }

    #[test]
    fn unresolved_blocked() {
        let u: Unresolved<i32> = Unresolved::Blocked {
            by: &["gap-1", "gap-2"],
            reason: "depends on other decisions",
        };
        assert!(!u.is_resolved());
        assert!(u.blocks_codegen());
        assert_eq!(u.resolved_value(), None);
        assert_eq!(u.reason(), Some("depends on other decisions"));
    }
}
