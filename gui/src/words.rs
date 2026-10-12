//! Turns events into short plain sentences for the status strip.
//!
//! The wording of questions and findings is already plain (the engine checks it); this only
//! picks one line per event. Events that are bookkeeping give `None`.

use nocodo_types::event::{Event, EventKind};

/// Longest prompt echoed in the strip, in characters.
const PROMPT_MAX: usize = 70;

fn shorten(text: &str, max: usize) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= max {
        return one_line;
    }
    let cut: String = one_line.chars().take(max).collect();
    format!("{}...", cut.trim_end())
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// One line for an event, or `None` when it is not worth showing.
pub fn event_words(event: &Event) -> Option<String> {
    let line = match &event.kind {
        EventKind::ProjectCreated { name, .. } => format!("Project \"{name}\" created."),
        EventKind::UserPrompt { text } => format!("You asked: {}", shorten(text, PROMPT_MAX)),
        EventKind::PromptFailed { detail, .. } => {
            format!("The assistant could not finish that request. {detail}")
        }
        EventKind::ChangeSetProposed { .. } => "The assistant made a plan for your lists.".into(),
        EventKind::ChangeSetRejected { .. } => {
            "The plan did not pass the checks, so nothing was changed.".into()
        }
        EventKind::ChangeSetAccepted { .. } => "The plan passed all the checks.".into(),
        EventKind::QuestionsAsked { questions, .. } => format!(
            "The assistant has {} for you.",
            plural(questions.len(), "question", "questions")
        ),
        EventKind::QuestionAnswered { detail, .. } => detail.clone(),
        EventKind::MigrationApplied { .. } => "Your lists were updated.".into(),
        EventKind::MigrationFailed { restored, .. } => {
            if *restored {
                "Your lists could not be updated. Your data was put back as it was.".into()
            } else {
                "Your lists could not be updated.".into()
            }
        }
        EventKind::DatabaseChecked { check } if !check.passed => {
            format!("A final check found a problem. {}", check.detail)
        }
        EventKind::SampleRowsAdded { rows, .. } => {
            let total: u64 = rows.values().sum();
            format!("Added {total} sample rows.")
        }
        EventKind::QuestionsRejected { .. }
        | EventKind::QuestionsDropped { .. }
        | EventKind::DatabaseChecked { .. } => return None,
    };
    Some(line)
}

/// Whether the event ends a run badly, so the GUI should raise an alert.
pub fn failure_words(event: &Event) -> Option<String> {
    match &event.kind {
        EventKind::PromptFailed { detail, .. } => Some(format!(
            "The assistant could not finish that request. {detail}"
        )),
        EventKind::MigrationFailed { restored, .. } => Some(if *restored {
            "Your lists could not be updated, and your data was put back as it was. \
             Try again, or describe the change differently."
                .into()
        } else {
            "Your lists could not be updated. Check the project folder before going on.".into()
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nocodo_types::event::Actor;
    use std::collections::BTreeMap;

    fn ev(kind: EventKind) -> Event {
        Event {
            seq: 1,
            at: "2026-10-01T00:00:00Z".parse().unwrap(),
            actor: Actor::System,
            kind,
        }
    }

    #[test]
    fn prompts_are_shortened_to_one_line() {
        let long = format!("Keep track\nof {}", "money ".repeat(40));
        let line = event_words(&ev(EventKind::UserPrompt { text: long })).unwrap();
        assert!(line.starts_with("You asked: Keep track of money"));
        assert!(line.ends_with("..."));
        assert!(!line.contains('\n'));
        assert!(line.chars().count() < 100);
    }

    #[test]
    fn applied_and_sample_rows_use_plain_words() {
        let applied = event_words(&ev(EventKind::MigrationApplied {
            migration: "0002_x".into(),
            backup: None,
            rows_before: BTreeMap::new(),
            rows_after: BTreeMap::new(),
            detail: "d".into(),
        }))
        .unwrap();
        assert_eq!(applied, "Your lists were updated.");
        let rows = event_words(&ev(EventKind::SampleRowsAdded {
            rows: BTreeMap::from([("a".into(), 3), ("b".into(), 2)]),
            detail: "d".into(),
        }))
        .unwrap();
        assert_eq!(rows, "Added 5 sample rows.");
        for line in [applied, rows] {
            assert!(!line.to_lowercase().contains("migration"));
        }
    }

    #[test]
    fn bookkeeping_events_are_skipped_and_failures_flagged() {
        let dropped = ev(EventKind::QuestionsDropped {
            question_ids: vec![],
            detail: "d".into(),
        });
        assert_eq!(event_words(&dropped), None);
        let failed = ev(EventKind::PromptFailed {
            prompt_seq: 2,
            error: "429".into(),
            run_id: None,
            detail: "Wait a minute and try again.".into(),
        });
        assert!(event_words(&failed).unwrap().contains("Wait a minute"));
        assert!(failure_words(&failed).unwrap().contains("Wait a minute"));
        assert_eq!(failure_words(&dropped), None);
    }

    #[test]
    fn question_count_is_singular_or_plural() {
        assert_eq!(plural(1, "question", "questions"), "1 question");
        assert_eq!(plural(3, "question", "questions"), "3 questions");
    }
}
