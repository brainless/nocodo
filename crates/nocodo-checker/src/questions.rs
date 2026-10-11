//! Rules for a round of questions from the Schema agent, run before anything is recorded.

use nocodo_types::event::Finding;
use nocodo_types::question::{Answer, Question, tech_words_in};

/// Most questions one round may hold.
pub const MAX_QUESTIONS: usize = 8;

/// A question answered earlier in the project, with its answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsweredQuestion {
    pub question: Question,
    pub answer: Answer,
    /// True when it was answered during the prompt run being checked.
    /// Then the agent must return a change set, not more questions.
    pub in_this_prompt: bool,
}

fn finding(code: &str, message: String) -> Finding {
    Finding {
        code: code.to_string(),
        message,
    }
}

fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Checks one round of questions. Returns no findings when the round may be asked.
pub fn check_questions(
    questions: &[Question],
    answered_so_far: &[AnsweredQuestion],
) -> Vec<Finding> {
    let mut out = Vec::new();

    if answered_so_far.iter().any(|a| a.in_this_prompt) && !questions.is_empty() {
        out.push(finding(
            "questions_after_answers",
            "The user already answered a round of questions for this prompt. Return a change set now and do not ask again.".into(),
        ));
    }
    if questions.len() > MAX_QUESTIONS {
        out.push(finding(
            "too_many_questions",
            format!(
                "{} questions in one round; ask at most {MAX_QUESTIONS}, the most important first.",
                questions.len()
            ),
        ));
    }

    for (i, q) in questions.iter().enumerate() {
        let label = format!("Question {} (\"{}\")", i + 1, q.text);
        for (what, text) in std::iter::once(("the question text", &q.text))
            .chain(q.choices.iter().map(|c| ("a choice", c)))
        {
            for word in tech_words_in(text) {
                out.push(finding(
                    "technical_word",
                    format!(
                        "{label}: {what} uses the technical word \"{word}\" in \"{text}\". Use words the user would say about their own things."
                    ),
                ));
            }
        }

        let n = q.choices.len();
        if !(2..=5).contains(&n) {
            out.push(finding(
                "choices_count",
                format!("{label} has {n} choices; give 2 to 5."),
            ));
        }
        let mut seen: Vec<String> = Vec::new();
        for c in &q.choices {
            let key = c.trim().to_lowercase();
            if seen.contains(&key) {
                out.push(finding(
                    "duplicate_choice",
                    format!("{label}: the choice \"{c}\" appears more than once."),
                ));
            } else {
                seen.push(key);
            }
        }

        match q.default {
            Some(d) if d < 1 || d > n => out.push(finding(
                "bad_default",
                format!("{label}: default {d} is outside 1 to {n}; choices are numbered from 1."),
            )),
            None if !q.blocking => out.push(finding(
                "bad_default",
                format!("{label} is not blocking, so it needs a default choice."),
            )),
            _ => {}
        }

        let norm = normalize(&q.text);
        if let Some(prev) = answered_so_far
            .iter()
            .find(|a| normalize(&a.question.text) == norm)
        {
            let chosen = prev
                .question
                .choices
                .get(prev.answer.choice.wrapping_sub(1))
                .map_or("(unknown choice)", String::as_str);
            out.push(finding(
                "already_answered",
                format!(
                    "{label} was already answered as \"{chosen}\" (choice {} of {}). Do not ask it again.",
                    prev.answer.choice,
                    prev.question.choices.len()
                ),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(text: &str, choices: &[&str]) -> Question {
        Question {
            id: String::new(),
            asked_by: "schema".into(),
            text: text.into(),
            choices: choices.iter().map(|c| c.to_string()).collect(),
            default: Some(1),
            blocking: false,
            why: String::new(),
        }
    }

    fn good() -> Question {
        q("Can a spend be for the whole family?", &["Yes", "No"])
    }

    fn codes(f: &[Finding]) -> Vec<&str> {
        f.iter().map(|f| f.code.as_str()).collect()
    }

    fn answered(question: Question, choice: usize, in_this_prompt: bool) -> AnsweredQuestion {
        AnsweredQuestion {
            question,
            answer: Answer {
                question_id: "q1".into(),
                choice,
                by_default: false,
            },
            in_this_prompt,
        }
    }

    #[test]
    fn good_round_passes() {
        assert!(check_questions(&[good()], &[]).is_empty());
        let mut blocking = good();
        blocking.default = None;
        blocking.blocking = true;
        assert!(check_questions(&[blocking], &[]).is_empty());
    }

    #[test]
    fn technical_word() {
        let mut bad = good();
        bad.text = "Should each spend have its own ID?".into();
        let f = check_questions(&[bad], &[]);
        assert_eq!(codes(&f), vec!["technical_word"]);
        assert!(f[0].message.contains("\"id\""));
        let mut bad = good();
        bad.choices[1] = "No, one table".into();
        assert_eq!(codes(&check_questions(&[bad], &[])), vec!["technical_word"]);
    }

    #[test]
    fn choices_count() {
        let one = q("Pick?", &["Only"]);
        assert_eq!(codes(&check_questions(&[one], &[])), vec!["choices_count"]);
        let six = q("Pick?", &["a", "b", "c", "d", "e", "f"]);
        assert_eq!(codes(&check_questions(&[six], &[])), vec!["choices_count"]);
        let five = q("Pick?", &["a", "b", "c", "d", "e"]);
        assert!(check_questions(&[five], &[]).is_empty());
    }

    #[test]
    fn duplicate_choice() {
        let dup = q("Pick?", &["Yes", " yes "]);
        assert_eq!(
            codes(&check_questions(&[dup], &[])),
            vec!["duplicate_choice"]
        );
    }

    #[test]
    fn bad_default() {
        for d in [0, 3] {
            let mut bad = good();
            bad.default = Some(d);
            assert_eq!(codes(&check_questions(&[bad], &[])), vec!["bad_default"]);
        }
        let mut missing = good();
        missing.default = None;
        assert_eq!(
            codes(&check_questions(&[missing], &[])),
            vec!["bad_default"]
        );
    }

    #[test]
    fn too_many_questions() {
        let make = |n: usize| -> Vec<Question> {
            (0..n)
                .map(|i| {
                    q(
                        &format!("Question number {}?", "x".repeat(i + 1)),
                        &["Yes", "No"],
                    )
                })
                .collect()
        };
        assert!(check_questions(&make(8), &[]).is_empty());
        assert_eq!(
            codes(&check_questions(&make(9), &[])),
            vec!["too_many_questions"]
        );
    }

    #[test]
    fn already_answered() {
        let prev = answered(good(), 2, false);
        assert!(
            check_questions(
                &[q("Something else?", &["a", "b"])],
                std::slice::from_ref(&prev)
            )
            .is_empty()
        );
        let mut again = good();
        again.text = "  can a SPEND be for the  whole family? ".into();
        let f = check_questions(&[again], &[prev]);
        assert_eq!(codes(&f), vec!["already_answered"]);
        assert!(f[0].message.contains("\"No\""));
    }

    #[test]
    fn questions_after_answers() {
        let other = q("Something else?", &["a", "b"]);
        let earlier_prompt = answered(good(), 1, false);
        assert!(check_questions(std::slice::from_ref(&other), &[earlier_prompt]).is_empty());
        let this_prompt = answered(good(), 1, true);
        assert_eq!(
            codes(&check_questions(
                &[other],
                std::slice::from_ref(&this_prompt)
            )),
            vec!["questions_after_answers"]
        );
        assert!(check_questions(&[], &[this_prompt]).is_empty());
    }
}
