use super::primitives::AtLeastOne;

#[derive(Debug, Clone)]
pub enum Provenance {
    Conversation {
        id: &'static str,
        excerpt: &'static str,
    },
    File {
        path: &'static str,
        lines: (u32, u32),
        excerpt: &'static str,
    },
}

impl Provenance {
    pub fn excerpt(&self) -> &'static str {
        match self {
            Provenance::Conversation { excerpt, .. } => excerpt,
            Provenance::File { excerpt, .. } => excerpt,
        }
    }
}

pub struct PrdValue<T> {
    pub value: T,
    pub provenance: AtLeastOne<Provenance>,
}

impl<T> PrdValue<T> {
    pub fn new(value: T, provenance: AtLeastOne<Provenance>) -> Self {
        Self { value, provenance }
    }
}
