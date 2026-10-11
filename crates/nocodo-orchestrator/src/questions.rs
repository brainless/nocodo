//! Question state derived from `events.jsonl`, and the wording of the removal
//! confirmation. Pure functions over the event log; nothing is stored elsewhere.

use std::collections::BTreeMap;

use nocodo_checker::AnsweredQuestion;
use nocodo_types::change::{ChangeSet, SchemaChange};
use nocodo_types::event::{Event, EventKind};
use nocodo_types::naming::{column_words, singular, words};
use nocodo_types::question::{Answer, Question, tech_words_in};
use nocodo_types::schema::SchemaSpec;

/// Asker of the removal confirmation.
pub(crate) const SYSTEM: &str = "system";
/// Asker of the Schema agent's questions.
pub(crate) const SCHEMA: &str = "schema";

pub(crate) fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Every answer recorded so far, by question ID.
pub(crate) fn answers(events: &[Event]) -> BTreeMap<String, Answer> {
    events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::QuestionAnswered { answer, .. } => {
                Some((answer.question_id.clone(), answer.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Every question asked so far with the event it was asked in, in order.
pub(crate) fn asked(events: &[Event]) -> Vec<(&EventKind, &Question)> {
    events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::QuestionsAsked { questions, .. } => Some((&e.kind, questions)),
            _ => None,
        })
        .flat_map(|(k, qs)| qs.iter().map(move |q| (k, q)))
        .collect()
}

/// The questions still waiting for an answer, in the order they were asked.
pub(crate) fn open_questions(events: &[Event]) -> Vec<Question> {
    let done = answers(events);
    asked(events)
        .into_iter()
        .map(|(_, q)| q)
        .filter(|q| !done.contains_key(&q.id))
        .cloned()
        .collect()
}

fn number_of(id: &str) -> u64 {
    id.strip_prefix('q')
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// The next unused question number (IDs dropped as duplicates count as used).
pub(crate) fn next_number(events: &[Event]) -> u64 {
    let mut max = 0;
    for e in events {
        match &e.kind {
            EventKind::QuestionsAsked { questions, .. } => {
                max = questions
                    .iter()
                    .map(|q| number_of(&q.id))
                    .fold(max, u64::max);
            }
            EventKind::QuestionsDropped { question_ids, .. } => {
                max = question_ids
                    .iter()
                    .map(|i| number_of(i))
                    .fold(max, u64::max);
            }
            _ => {}
        }
    }
    max + 1
}

/// The Schema agent's answered questions, for the checker and the agent's input.
/// `in_this_prompt` marks the rounds that belong to `prompt_seq`.
pub(crate) fn answered_questions(events: &[Event], prompt_seq: u64) -> Vec<AnsweredQuestion> {
    let done = answers(events);
    asked(events)
        .into_iter()
        .filter(|(_, q)| q.asked_by == SCHEMA)
        .filter_map(|(kind, q)| {
            let answer = done.get(&q.id)?.clone();
            let in_this_prompt = matches!(
                kind,
                EventKind::QuestionsAsked { prompt_seq: Some(p), .. } if *p == prompt_seq
            );
            Some(AnsweredQuestion {
                question: q.clone(),
                answer,
                in_this_prompt,
            })
        })
        .collect()
}

/// How many attempts the log already holds for a prompt (proposals, rejected
/// and asked rounds), so a resumed run numbers its attempts on from there.
pub(crate) fn attempts_used(events: &[Event], prompt_seq: u64) -> u32 {
    let n = events
        .iter()
        .filter(|e| match &e.kind {
            EventKind::ChangeSetProposed {
                prompt_seq: Some(p),
                ..
            } => *p == prompt_seq,
            EventKind::QuestionsRejected { prompt_seq: p, .. } => *p == prompt_seq,
            EventKind::QuestionsAsked {
                prompt_seq: Some(p),
                ..
            } => *p == prompt_seq,
            _ => false,
        })
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn trim_sentence(s: &str) -> &str {
    s.trim().trim_end_matches('.')
}

/// One line per removed thing, in domain words, from the descriptions in `schema.json`.
fn removal_lines(current: &SchemaSpec, cs: &ChangeSet, with_descriptions: bool) -> Vec<String> {
    let mut lines = Vec::new();
    for change in &cs.changes {
        match change {
            SchemaChange::DropColumn { table, column } => {
                let found = current
                    .tables
                    .iter()
                    .find(|t| &t.name == table)
                    .and_then(|t| t.columns.iter().find(|c| &c.name == column));
                let what = found.map_or_else(|| words(column), column_words);
                let note = found
                    .map(|c| trim_sentence(&c.description))
                    .filter(|d| with_descriptions && !d.is_empty())
                    .map(|d| format!(" ({d})"))
                    .unwrap_or_default();
                lines.push(format!(
                    "the {what} you have entered on each {}{note}",
                    words(&singular(table))
                ));
            }
            SchemaChange::DropTable { table } => lines.push(format!(
                "every {} you have entered, and everything recorded about them",
                words(&singular(table))
            )),
            _ => {}
        }
    }
    lines
}

/// The confirmation question for a removal, built by plain code. It must pass
/// the same word check as the agent's questions; if a description from
/// `schema.json` brings in a technical word, the descriptions are left out.
pub(crate) fn confirmation_text(current: &SchemaSpec, cs: &ChangeSet) -> String {
    let build = |with_descriptions: bool| {
        let lines = removal_lines(current, cs, with_descriptions);
        match lines.as_slice() {
            [one] => format!("This will forget {one}. Continue?"),
            many => format!(
                "This will forget:\n{}\nContinue?",
                many.iter()
                    .map(|l| format!("- {l}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        }
    };
    let text = build(true);
    if tech_words_in(&text).is_empty() {
        text
    } else {
        build(false)
    }
}
