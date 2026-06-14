pub struct AtLeastOne<T: 'static> {
    pub head: T,
    pub tail: &'static [T],
}

impl<T> AtLeastOne<T> {
    pub fn all(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.head).chain(self.tail.iter())
    }

    pub fn contains<F: Fn(&T) -> bool>(&self, f: F) -> bool {
        self.all().any(f)
    }

    pub fn len(&self) -> usize {
        1 + self.tail.len()
    }
}

pub enum Unresolved<T> {
    Resolved(T),
    Pending {
        reason: &'static str,
        provenance: AtLeastOne<super::provenance::Provenance>,
    },
    Blocked {
        by: &'static [&'static str],
        reason: &'static str,
    },
}

impl<T> Unresolved<T> {
    pub fn is_resolved(&self) -> bool {
        matches!(self, Unresolved::Resolved(_))
    }

    pub fn blocks_codegen(&self) -> bool {
        !self.is_resolved()
    }

    pub fn resolved_value(&self) -> Option<&T> {
        match self {
            Unresolved::Resolved(v) => Some(v),
            _ => None,
        }
    }

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
