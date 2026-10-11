//! Questions to the user, their answers, and the shared list of technical words.
//!
//! Questions live only in `events.jsonl`; their status is derived from the log.
//! Choice numbers start at 1 everywhere.

use serde::{Deserialize, Serialize};

/// Words that must never appear in text shown to the user.
/// Matching ignores case and works on whole words (see [`tech_words_in`]).
pub const TECH_WORDS: &[&str] = &[
    "table",
    "column",
    "row",
    "sql",
    "database",
    "schema",
    "id",
    "api",
    "json",
    "migration",
    "query",
    "index",
    "foreign key",
    "nullable",
    "integer",
    "boolean",
    "string",
    "field",
];

/// Returns the [`TECH_WORDS`] found in `text`, each at most once, in list order.
///
/// Whole words only ("identity" and "rowing" are not flagged), ignoring case.
/// A plural with a trailing "s" counts as the word ("tables", "IDs").
pub fn tech_words_in(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let same = |word: &str, tech: &str| word == tech || word.strip_suffix('s') == Some(tech);
    TECH_WORDS
        .iter()
        .copied()
        .filter(|tech| {
            let parts: Vec<&str> = tech.split(' ').collect();
            words.windows(parts.len()).any(|w| {
                let last = parts.len() - 1;
                w.iter().zip(&parts).enumerate().all(|(i, (word, part))| {
                    if i == last {
                        same(word, part)
                    } else {
                        word == part
                    }
                })
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    /// `q1`, `q2`, ... one sequence per project, assigned by the orchestrator.
    pub id: String,
    /// `schema` (the agent) or `system` (plain code, such as the destructive confirmation).
    pub asked_by: String,
    /// The question, in domain language.
    pub text: String,
    /// 2 to 5 options.
    pub choices: Vec<String>,
    /// Number (from 1) of the choice used when the user accepts the defaults.
    /// `None` only for blocking questions.
    #[serde(default)]
    pub default: Option<usize>,
    /// Blocking questions must be answered explicitly; `--defaults` skips them.
    #[serde(default)]
    pub blocking: bool,
    /// For the reviewer, not shown to the user: what depends on the answer.
    /// Technical words are allowed here.
    #[serde(default)]
    pub why: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub question_id: String,
    /// Numbered from 1, as typed in `answer`.
    pub choice: usize,
    pub by_default: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_words_only() {
        assert!(tech_words_in("identity of the rowing club").is_empty());
        assert_eq!(tech_words_in("What is your ID?"), vec!["id"]);
        assert_eq!(tech_words_in("Table of Contents"), vec!["table"]);
        assert_eq!(tech_words_in("the tables and IDs"), vec!["table", "id"]);
        assert_eq!(tech_words_in("a Foreign  Key here"), vec!["foreign key"]);
        assert!(tech_words_in("a foreign friend with a key").is_empty());
        assert!(tech_words_in("type of expense, keep a list, my record").is_empty());
    }

    #[test]
    fn question_round_trips_and_old_fields_default() {
        let q = Question {
            id: "q1".into(),
            asked_by: "schema".into(),
            text: "Is a spend for the whole family?".into(),
            choices: vec!["Yes".into(), "No".into()],
            default: Some(1),
            blocking: false,
            why: "one or many".into(),
        };
        let json = serde_json::to_string(&q).unwrap();
        assert_eq!(serde_json::from_str::<Question>(&json).unwrap(), q);
        let min = r#"{"id":"q2","asked_by":"system","text":"t","choices":["a","b"]}"#;
        let q2: Question = serde_json::from_str(min).unwrap();
        assert_eq!((q2.default, q2.blocking), (None, false));
        let a = Answer {
            question_id: "q1".into(),
            choice: 1,
            by_default: true,
        };
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Answer>(&json).unwrap(), a);
    }
}
