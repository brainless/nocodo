//! What the window shows and when it asks the server again. No drawing and no I/O here, so the
//! logic can be tested without a window or a server.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use nocodo_types::event::EventKind;
use nocodo_types::outcome::RowPage;
use nocodo_types::question::Question;

use crate::client::{Msg, ProjectEntry, Summary};
use crate::words::{event_words, failure_words};

/// Status lines kept for the strip.
pub const MAX_LINES: usize = 8;
/// Rows asked for per page of a table.
pub const PAGE_SIZE: u64 = 50;
/// `asked_by` of questions that plain code asks (the removal confirmation).
const SYSTEM: &str = "system";
const POLL_IDLE: Duration = Duration::from_secs(3);
const POLL_RUNNING: Duration = Duration::from_millis(800);

/// Something the window must do after a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Save this folder name as the remembered project.
    Remember(String),
    /// Ask for the project list again.
    RefreshProjects,
}

/// A request that is due.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Poll {
    Events {
        project: String,
        after: u64,
    },
    Run {
        project: String,
        run_id: String,
    },
    Questions {
        project: String,
    },
    Summary {
        project: String,
    },
    Rows {
        project: String,
        table: String,
        offset: u64,
    },
}

/// Offset of the last page of `total` rows.
pub fn last_page_offset(total: u64, size: u64) -> u64 {
    if total == 0 || size == 0 {
        0
    } else {
        (total - 1) / size * size
    }
}

/// "Rows 1-50 of 120", or a plain sentence for an empty list.
pub fn page_label(page: &RowPage) -> String {
    if page.total == 0 {
        return "No rows yet".into();
    }
    let first = page.offset + 1;
    let last = page.offset + page.rows.len() as u64;
    format!("Rows {first}-{last} of {}", page.total)
}

#[derive(Default)]
pub struct Model {
    pub projects: Vec<ProjectEntry>,
    pub selected: Option<String>,
    /// Remembered folder, picked once the list arrives.
    remembered: Option<String>,
    /// Plain-language lines about what happened, oldest first.
    pub lines: Vec<String>,
    /// Shown as an alert until dismissed.
    pub error: Option<String>,
    /// The prompt run being followed.
    pub run: Option<String>,
    /// Questions waiting for an answer, blocking ones first.
    pub questions: Vec<Question>,
    /// Choices picked but not sent yet (question ID to choice number from 1).
    pub picks: BTreeMap<String, usize>,
    pub summary: Option<Summary>,
    /// The list shown in the data view.
    pub table: Option<String>,
    pub offset: u64,
    pub page: Option<RowPage>,
    /// The last run failed (or the model call did not finish): offer Retry.
    pub retry: bool,
    sending: bool,
    questions_dirty: bool,
    summary_dirty: bool,
    rows_dirty: bool,
    questions_inflight: bool,
    summary_inflight: bool,
    rows_inflight: bool,
    last_seq: u64,
    loaded: bool,
    events_inflight: bool,
    run_inflight: bool,
    events_polled: Option<Instant>,
    run_polled: Option<Instant>,
}

impl Model {
    pub fn new(remembered: Option<String>) -> Self {
        Self {
            remembered,
            ..Self::default()
        }
    }

    pub fn is_running(&self) -> bool {
        self.run.is_some()
    }

    pub fn can_send(&self) -> bool {
        self.selected.is_some() && !self.is_running() && !self.sending
    }

    pub fn can_apply(&self) -> bool {
        self.can_send()
            && self
                .summary
                .as_ref()
                .is_some_and(|summary| !summary.pending.is_empty())
    }

    /// Call right before a request that starts a run, so a second click cannot send it twice.
    pub fn begin_send(&mut self) {
        self.sending = true;
    }

    /// Questions the assistant asks, shown under the prompt box.
    pub fn open_questions(&self) -> Vec<&Question> {
        self.questions
            .iter()
            .filter(|q| q.asked_by != SYSTEM)
            .collect()
    }

    /// The removal confirmation, when one waits: it is shown as a modal.
    pub fn confirmation(&self) -> Option<&Question> {
        self.questions.iter().find(|q| q.asked_by == SYSTEM)
    }

    pub fn can_use_defaults(&self) -> bool {
        self.open_questions().iter().any(|q| q.default.is_some())
    }

    /// Picks a choice (numbered from 1) for an open question. Others are ignored.
    pub fn pick(&mut self, question: &str, choice: usize) {
        let valid = self
            .questions
            .iter()
            .any(|q| q.id == question && (1..=q.choices.len()).contains(&choice));
        if valid {
            self.picks.insert(question.to_string(), choice);
        }
    }

    /// The `POST /answers` body for the picks made so far, which are then cleared.
    pub fn take_answers(&mut self) -> Option<serde_json::Value> {
        let picks = std::mem::take(&mut self.picks);
        let body: serde_json::Map<String, serde_json::Value> = picks
            .into_iter()
            .filter(|(id, _)| self.questions.iter().any(|q| &q.id == id))
            .map(|(id, choice)| (id, choice.into()))
            .collect();
        (!body.is_empty()).then_some(serde_json::Value::Object(body))
    }

    /// The `POST /answers` body for the removal confirmation's choice.
    pub fn confirmation_answer(&self, choice: usize) -> Option<serde_json::Value> {
        let q = self.confirmation()?;
        (1..=q.choices.len())
            .contains(&choice)
            .then(|| serde_json::json!({ q.id.clone(): choice }))
    }

    /// Names of the lists with data, for the tabs.
    pub fn table_names(&self) -> Vec<String> {
        self.summary
            .as_ref()
            .map(|s| s.tables.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn table_index(&self) -> usize {
        let names = self.table_names();
        self.table
            .as_ref()
            .and_then(|t| names.iter().position(|n| n == t))
            .unwrap_or(0)
    }

    /// Shows another list, from its first page.
    pub fn choose_table(&mut self, name: &str) {
        if self.table.as_deref() == Some(name) {
            return;
        }
        self.table = Some(name.to_string());
        self.offset = 0;
        self.page = None;
        self.rows_dirty = true;
    }

    /// The page being shown, only when it is of the chosen list.
    pub fn current_page(&self) -> Option<&RowPage> {
        self.page
            .as_ref()
            .filter(|p| self.table.as_deref() == Some(p.table.as_str()))
    }

    pub fn can_prev(&self) -> bool {
        self.current_page().is_some_and(|p| p.offset > 0)
    }

    pub fn can_next(&self) -> bool {
        self.current_page()
            .is_some_and(|p| p.offset + PAGE_SIZE < p.total)
    }

    pub fn prev_page(&mut self) {
        if self.can_prev() {
            self.offset = self.offset.saturating_sub(PAGE_SIZE);
            self.rows_dirty = true;
        }
    }

    pub fn next_page(&mut self) {
        if self.can_next() {
            self.offset += PAGE_SIZE;
            self.rows_dirty = true;
        }
    }

    fn started(&mut self, project: String, result: Result<String, String>) {
        self.sending = false;
        if self.selected.as_deref() != Some(&project) {
            return;
        }
        match result {
            Ok(run_id) => {
                self.error = None;
                self.retry = false;
                self.run = Some(run_id);
                self.events_polled = None;
                self.run_polled = None;
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Dropdown label.
    pub fn project_label(&self) -> &str {
        self.selected.as_deref().unwrap_or("Select a project")
    }

    /// Picks a project (the user's choice). Resets what was shown for the previous one.
    pub fn select(&mut self, folder: &str) -> Vec<Effect> {
        if self.selected.as_deref() == Some(folder) {
            return Vec::new();
        }
        self.selected = Some(folder.to_string());
        self.lines.clear();
        self.error = None;
        self.run = None;
        self.last_seq = 0;
        self.loaded = false;
        self.events_inflight = false;
        self.run_inflight = false;
        self.events_polled = None;
        self.run_polled = None;
        self.questions.clear();
        self.picks.clear();
        self.summary = None;
        self.table = None;
        self.offset = 0;
        self.page = None;
        self.retry = false;
        self.sending = false;
        self.questions_inflight = false;
        self.summary_inflight = false;
        self.rows_inflight = false;
        self.questions_dirty = true;
        self.summary_dirty = true;
        self.rows_dirty = false;
        vec![Effect::Remember(folder.to_string())]
    }

    fn push_line(&mut self, line: String) {
        self.lines.push(line);
        if self.lines.len() > MAX_LINES {
            self.lines.remove(0);
        }
    }

    pub fn handle(&mut self, msg: Msg) -> Vec<Effect> {
        let mut effects = Vec::new();
        match msg {
            Msg::Projects(Ok(list)) => {
                self.projects = list;
                if self.selected.is_none()
                    && let Some(r) = self.remembered.take()
                    && self.projects.iter().any(|p| p.is_usable() && p.id() == r)
                {
                    // Restoring a choice is not a new choice: nothing to save.
                    let _ = self.select(&r);
                }
            }
            Msg::Projects(Err(e)) => self.error = Some(e),
            Msg::Created(Ok(folder)) => {
                effects.extend(self.select(&folder));
                effects.push(Effect::RefreshProjects);
            }
            Msg::Created(Err(e)) => self.error = Some(e),
            Msg::PromptSent { project, result } | Msg::Started { project, result } => {
                self.started(project, result);
            }
            Msg::Questions { project, result } => {
                if self.selected.as_deref() != Some(&project) {
                    return effects;
                }
                self.questions_inflight = false;
                match result {
                    Ok(mut list) => {
                        // Blocking ones first; the order is otherwise the server's.
                        list.sort_by_key(|q| !q.blocking);
                        self.picks.retain(|id, _| list.iter().any(|q| &q.id == id));
                        self.questions = list;
                    }
                    Err(e) => {
                        if self.error.is_none() {
                            self.error = Some(e);
                        }
                    }
                }
            }
            Msg::Summary { project, result } => {
                if self.selected.as_deref() != Some(&project) {
                    return effects;
                }
                self.summary_inflight = false;
                match result {
                    Ok(summary) => {
                        let keep = self
                            .table
                            .as_ref()
                            .filter(|t| summary.tables.contains_key(*t))
                            .cloned();
                        let table = keep.or_else(|| summary.tables.keys().next().cloned());
                        if table != self.table {
                            self.table = table;
                            self.offset = 0;
                            self.page = None;
                        }
                        self.rows_dirty = self.table.is_some();
                        self.summary = Some(summary);
                    }
                    Err(e) => {
                        if self.error.is_none() {
                            self.error = Some(e);
                        }
                    }
                }
            }
            Msg::Rows {
                project,
                table,
                offset,
                result,
            } => {
                if self.selected.as_deref() != Some(&project) {
                    return effects;
                }
                self.rows_inflight = false;
                if self.table.as_deref() != Some(&table) || self.offset != offset {
                    return effects;
                }
                match result {
                    Ok(page) => {
                        // The list got shorter (rows removed): go to the last page that exists.
                        if page.total > 0 && offset >= page.total {
                            self.offset = last_page_offset(page.total, PAGE_SIZE);
                            self.rows_dirty = true;
                        } else {
                            self.page = Some(page);
                        }
                    }
                    Err(e) => {
                        if self.error.is_none() {
                            self.error = Some(e);
                        }
                    }
                }
            }
            Msg::Events { project, result } => {
                if self.selected.as_deref() != Some(&project) {
                    return effects;
                }
                self.events_inflight = false;
                match result {
                    Ok(page) => {
                        let fresh = self.loaded;
                        let seen = self.last_seq;
                        for event in page.events.iter().filter(|e| e.seq > seen) {
                            match event.kind {
                                EventKind::PromptFailed { .. }
                                | EventKind::MigrationFailed { .. } => self.retry = true,
                                EventKind::UserPrompt { .. }
                                | EventKind::ChangeSetProposed { .. }
                                | EventKind::QuestionsAsked { .. }
                                | EventKind::MigrationApplied { .. } => self.retry = false,
                                _ => {}
                            }
                            if fresh {
                                self.questions_dirty = true;
                            }
                            if let Some(line) = event_words(event) {
                                self.push_line(line);
                            }
                            if fresh {
                                if let Some(alert) = failure_words(event)
                                    && self.error.is_none()
                                {
                                    self.error = Some(alert);
                                }
                                if matches!(
                                    event.kind,
                                    EventKind::MigrationApplied { .. }
                                        | EventKind::SampleRowsAdded { .. }
                                ) {
                                    self.summary_dirty = true;
                                    effects.push(Effect::RefreshProjects);
                                }
                            }
                        }
                        self.last_seq = self.last_seq.max(page.last);
                        self.loaded = true;
                    }
                    Err(e) => {
                        if self.error.is_none() {
                            self.error = Some(e);
                        }
                    }
                }
            }
            Msg::Run {
                project,
                run_id,
                result,
            } => {
                if self.selected.as_deref() != Some(&project)
                    || self.run.as_deref() != Some(&run_id)
                {
                    return effects;
                }
                self.run_inflight = false;
                match result {
                    Ok(status) if status.status == "running" => {}
                    Ok(status) => {
                        self.run = None;
                        // Fetch the run's last events at once.
                        self.events_polled = None;
                        self.questions_dirty = true;
                        self.summary_dirty = true;
                        if status.status == "failed" {
                            self.retry = true;
                            self.error = Some(status.error.unwrap_or_else(|| {
                                "That request failed. Send it again in a moment.".into()
                            }));
                        }
                        effects.push(Effect::RefreshProjects);
                    }
                    Err(e) => {
                        if self.error.is_none() {
                            self.error = Some(e);
                        }
                    }
                }
            }
        }
        effects
    }

    /// The next request that is due, marking it in flight. At most one of each kind is out at a
    /// time.
    pub fn next_poll(&mut self, now: Instant) -> Option<Poll> {
        let project = self.selected.clone()?;
        let events_every = if self.is_running() {
            POLL_RUNNING
        } else {
            POLL_IDLE
        };
        let due = |last: Option<Instant>, every: Duration| {
            last.is_none_or(|t| now.duration_since(t) >= every)
        };
        if !self.events_inflight && due(self.events_polled, events_every) {
            self.events_inflight = true;
            self.events_polled = Some(now);
            return Some(Poll::Events {
                project,
                after: self.last_seq,
            });
        }
        if let Some(run_id) = self.run.clone()
            && !self.run_inflight
            && due(self.run_polled, POLL_RUNNING)
        {
            self.run_inflight = true;
            self.run_polled = Some(now);
            return Some(Poll::Run { project, run_id });
        }
        if self.questions_dirty && !self.questions_inflight {
            self.questions_dirty = false;
            self.questions_inflight = true;
            return Some(Poll::Questions { project });
        }
        if self.summary_dirty && !self.summary_inflight {
            self.summary_dirty = false;
            self.summary_inflight = true;
            return Some(Poll::Summary { project });
        }
        if self.rows_dirty
            && !self.rows_inflight
            && let Some(table) = self.table.clone()
        {
            self.rows_dirty = false;
            self.rows_inflight = true;
            return Some(Poll::Rows {
                project,
                table,
                offset: self.offset,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{EventsPage, RunStatus};
    use nocodo_types::event::{Actor, Event, EventKind};

    fn entry(folder: &str, error: Option<&str>) -> ProjectEntry {
        ProjectEntry {
            folder: Some(folder.into()),
            name: folder.into(),
            tables: Some(0),
            last_event: None,
            error: error.map(Into::into),
        }
    }

    fn ev(seq: u64, kind: EventKind) -> Event {
        Event {
            seq,
            at: "2026-10-01T00:00:00Z".parse().unwrap(),
            actor: Actor::System,
            kind,
        }
    }

    fn page(events: Vec<Event>) -> Result<EventsPage, String> {
        let last = events.last().map_or(0, |e| e.seq);
        Ok(EventsPage { events, last })
    }

    #[test]
    fn remembered_project_is_selected_when_it_exists_and_is_usable() {
        let mut m = Model::new(Some("b".into()));
        let fx = m.handle(Msg::Projects(Ok(vec![entry("a", None), entry("b", None)])));
        assert_eq!(m.selected.as_deref(), Some("b"));
        assert!(fx.is_empty());

        let mut m = Model::new(Some("gone".into()));
        m.handle(Msg::Projects(Ok(vec![entry("a", None)])));
        assert_eq!(m.selected, None);

        let mut m = Model::new(Some("a".into()));
        m.handle(Msg::Projects(Ok(vec![entry("a", Some("broken"))])));
        assert_eq!(m.selected, None);
    }

    #[test]
    fn choosing_saves_the_choice_and_resets_the_strip() {
        let mut m = Model::new(None);
        assert_eq!(m.select("a"), vec![Effect::Remember("a".into())]);
        m.lines.push("old".into());
        assert!(m.select("a").is_empty());
        assert_eq!(m.select("b"), vec![Effect::Remember("b".into())]);
        assert!(m.lines.is_empty());
    }

    #[test]
    fn new_project_is_selected_and_list_refreshed() {
        let mut m = Model::new(None);
        let fx = m.handle(Msg::Created(Ok("fresh".into())));
        assert_eq!(m.selected.as_deref(), Some("fresh"));
        assert_eq!(
            fx,
            vec![Effect::Remember("fresh".into()), Effect::RefreshProjects]
        );
        m.handle(Msg::Created(Err("It exists.".into())));
        assert_eq!(m.error.as_deref(), Some("It exists."));
    }

    #[test]
    fn first_events_fill_the_strip_without_alerts_then_new_failures_alert() {
        let mut m = Model::new(None);
        m.select("a");
        let failed = |seq| {
            ev(
                seq,
                EventKind::PromptFailed {
                    prompt_seq: 1,
                    error: "e".into(),
                    run_id: None,
                    detail: "Try again.".into(),
                },
            )
        };
        let poll = m.next_poll(Instant::now()).unwrap();
        assert_eq!(
            poll,
            Poll::Events {
                project: "a".into(),
                after: 0
            }
        );
        m.handle(Msg::Events {
            project: "a".into(),
            result: page(vec![
                ev(1, EventKind::UserPrompt { text: "hi".into() }),
                failed(2),
            ]),
        });
        assert_eq!(m.lines.len(), 2);
        assert_eq!(m.error, None, "history does not raise an alert");

        m.events_inflight = true;
        m.handle(Msg::Events {
            project: "a".into(),
            result: page(vec![failed(3)]),
        });
        assert_eq!(m.lines.len(), 3);
        assert!(m.error.as_deref().unwrap().contains("Try again."));
    }

    #[test]
    fn strip_keeps_only_the_latest_lines() {
        let mut m = Model::new(None);
        m.select("a");
        let events = (1..=12)
            .map(|i| {
                ev(
                    i,
                    EventKind::UserPrompt {
                        text: format!("p{i}"),
                    },
                )
            })
            .collect();
        m.handle(Msg::Events {
            project: "a".into(),
            result: page(events),
        });
        assert_eq!(m.lines.len(), MAX_LINES);
        assert!(m.lines.last().unwrap().ends_with("p12"));
    }

    #[test]
    fn events_for_another_project_are_ignored() {
        let mut m = Model::new(None);
        m.select("a");
        m.handle(Msg::Events {
            project: "b".into(),
            result: page(vec![ev(1, EventKind::UserPrompt { text: "x".into() })]),
        });
        assert!(m.lines.is_empty());
    }

    #[test]
    fn polling_follows_a_run_until_it_ends() {
        let mut m = Model::new(None);
        m.select("a");
        // The data requests have their own test.
        m.questions_dirty = false;
        m.summary_dirty = false;
        let t0 = Instant::now();
        assert!(matches!(m.next_poll(t0), Some(Poll::Events { .. })));
        // One events request at a time.
        assert_eq!(m.next_poll(t0), None);
        m.handle(Msg::Events {
            project: "a".into(),
            result: page(vec![]),
        });
        assert_eq!(
            m.next_poll(t0 + Duration::from_secs(1)),
            None,
            "idle is slower"
        );

        assert!(m.can_send());
        m.handle(Msg::PromptSent {
            project: "a".into(),
            result: Ok("r1".into()),
        });
        assert!(!m.can_send());
        let t1 = t0 + Duration::from_secs(1);
        assert!(matches!(m.next_poll(t1), Some(Poll::Events { .. })));
        assert_eq!(
            m.next_poll(t1),
            Some(Poll::Run {
                project: "a".into(),
                run_id: "r1".into()
            })
        );
        let fx = m.handle(Msg::Run {
            project: "a".into(),
            run_id: "r1".into(),
            result: Ok(RunStatus {
                status: "finished".into(),
                error: None,
            }),
        });
        assert!(m.can_send());
        assert_eq!(fx, vec![Effect::RefreshProjects]);
    }

    #[test]
    fn failed_run_and_busy_refusal_show_the_server_text() {
        let mut m = Model::new(None);
        m.select("a");
        m.handle(Msg::PromptSent {
            project: "a".into(),
            result: Err("This project is busy: `apply` is running.".into()),
        });
        assert!(!m.is_running());
        assert!(m.error.as_deref().unwrap().contains("busy"));

        m.error = None;
        m.handle(Msg::PromptSent {
            project: "a".into(),
            result: Ok("r2".into()),
        });
        m.handle(Msg::Run {
            project: "a".into(),
            run_id: "r2".into(),
            result: Ok(RunStatus {
                status: "failed".into(),
                error: Some("The model did not answer.".into()),
            }),
        });
        assert!(!m.is_running());
        assert_eq!(m.error.as_deref(), Some("The model did not answer."));
    }

    fn question(id: &str, blocking: bool, default: Option<usize>, asked_by: &str) -> Question {
        Question {
            id: id.into(),
            asked_by: asked_by.into(),
            text: format!("Question {id}?"),
            choices: vec!["One".into(), "Two".into(), "Three".into()],
            default,
            blocking,
            why: String::new(),
        }
    }

    fn summary_of(tables: &[(&str, u64)]) -> Summary {
        Summary {
            tables: tables.iter().map(|(n, c)| (n.to_string(), *c)).collect(),
            pending: Vec::new(),
            summary: "You keep lists.".into(),
        }
    }

    fn rows_page(table: &str, offset: u64, count: u64, total: u64) -> RowPage {
        use nocodo_types::outcome::{CellValue, RowColumn};
        RowPage {
            table: table.into(),
            columns: vec![RowColumn {
                name: "id".into(),
                label: "id".into(),
                column_type: nocodo_types::schema::ColumnType::Integer,
            }],
            rows: (0..count)
                .map(|i| vec![CellValue::Integer((offset + i + 1) as i64)])
                .collect(),
            total,
            offset,
            limit: PAGE_SIZE,
        }
    }

    /// A selected project whose first questions and summary requests were made and answered (empty).
    fn ready(project: &str) -> Model {
        let mut m = Model::new(None);
        m.select(project);
        let t = Instant::now();
        assert!(matches!(m.next_poll(t), Some(Poll::Events { .. })));
        assert!(matches!(m.next_poll(t), Some(Poll::Questions { .. })));
        assert!(matches!(m.next_poll(t), Some(Poll::Summary { .. })));
        m.handle(Msg::Questions {
            project: project.into(),
            result: Ok(vec![]),
        });
        m.handle(Msg::Summary {
            project: project.into(),
            result: Ok(summary_of(&[])),
        });
        m
    }

    #[test]
    fn questions_come_blocking_first_and_picks_follow_the_open_ones() {
        let mut m = ready("a");
        m.handle(Msg::Questions {
            project: "a".into(),
            result: Ok(vec![
                question("q1", false, Some(2), "schema"),
                question("q2", true, None, "schema"),
                question("q3", false, Some(1), "schema"),
            ]),
        });
        let ids: Vec<_> = m.open_questions().iter().map(|q| q.id.clone()).collect();
        assert_eq!(ids, ["q2", "q1", "q3"]);
        assert!(m.can_use_defaults());

        m.pick("q2", 3);
        m.pick("q1", 9); // no such choice
        m.pick("q9", 1); // no such question
        assert_eq!(m.picks.len(), 1);
        assert_eq!(m.take_answers(), Some(serde_json::json!({"q2": 3})));
        assert!(m.picks.is_empty());
        assert_eq!(m.take_answers(), None);

        // A pick for a question that is no longer open is dropped on the next load.
        m.pick("q1", 1);
        m.handle(Msg::Questions {
            project: "a".into(),
            result: Ok(vec![question("q3", false, Some(1), "schema")]),
        });
        assert!(m.picks.is_empty());
    }

    #[test]
    fn removal_confirmation_is_separate_and_answers_by_its_own_id() {
        let mut m = ready("a");
        m.handle(Msg::Questions {
            project: "a".into(),
            result: Ok(vec![question("q4", true, None, "system")]),
        });
        assert!(m.open_questions().is_empty());
        assert_eq!(m.confirmation().unwrap().id, "q4");
        assert!(!m.can_use_defaults());
        assert_eq!(m.confirmation_answer(2), Some(serde_json::json!({"q4": 2})));
        assert_eq!(m.confirmation_answer(4), None);
    }

    #[test]
    fn sending_blocks_a_second_send_until_the_server_answers() {
        let mut m = ready("a");
        assert!(m.can_send());
        m.begin_send();
        assert!(!m.can_send());
        m.handle(Msg::Started {
            project: "a".into(),
            result: Ok("r1".into()),
        });
        assert!(m.is_running());
        m.begin_send();
        m.handle(Msg::Started {
            project: "a".into(),
            result: Err("busy".into()),
        });
        assert_eq!(m.error.as_deref(), Some("busy"));
        assert!(m.is_running());
    }

    #[test]
    fn data_is_loaded_on_selection_and_reloaded_after_an_applied_change() {
        let mut m = ready("a");
        let t = Instant::now() + Duration::from_secs(10);
        m.handle(Msg::Summary {
            project: "a".into(),
            result: Ok(summary_of(&[("expenses", 2), ("people", 1)])),
        });
        assert_eq!(m.table.as_deref(), Some("expenses"));
        let first = Poll::Rows {
            project: "a".into(),
            table: "expenses".into(),
            offset: 0,
        };
        assert_eq!(m.next_poll(t), Some(first));
        m.handle(Msg::Rows {
            project: "a".into(),
            table: "expenses".into(),
            offset: 0,
            result: Ok(rows_page("expenses", 0, 2, 2)),
        });
        assert_eq!(m.current_page().unwrap().total, 2);
        // The first events (history) do not reload anything.
        m.handle(Msg::Events {
            project: "a".into(),
            result: page(vec![ev(
                1,
                EventKind::SampleRowsAdded {
                    rows: Default::default(),
                    detail: String::new(),
                },
            )]),
        });
        assert!(!matches!(m.next_poll(t), Some(Poll::Summary { .. })));

        // A change that is applied later reloads the summary and then the rows.
        let mut later = m.next_poll(t + Duration::from_secs(10));
        while let Some(p) = later {
            assert!(!matches!(p, Poll::Summary { .. }), "{p:?}");
            later = m.next_poll(t + Duration::from_secs(10));
        }
        m.events_inflight = false;
        m.handle(Msg::Events {
            project: "a".into(),
            result: page(vec![ev(
                2,
                EventKind::SampleRowsAdded {
                    rows: Default::default(),
                    detail: String::new(),
                },
            )]),
        });
        let mut seen_summary = false;
        let mut now = t + Duration::from_secs(20);
        while let Some(p) = m.next_poll(now) {
            seen_summary |= matches!(p, Poll::Summary { .. });
            now += Duration::from_millis(1);
        }
        assert!(seen_summary);
        m.handle(Msg::Summary {
            project: "a".into(),
            result: Ok(summary_of(&[("expenses", 5), ("people", 1)])),
        });
        assert_eq!(m.table.as_deref(), Some("expenses"), "the tab is kept");
        assert!(matches!(
            m.next_poll(now),
            Some(Poll::Rows { offset: 0, .. })
        ));
    }

    #[test]
    fn pending_changes_enable_apply_then_finished_run_loads_new_lists() {
        let mut m = ready("a");
        assert!(!m.can_apply());
        let mut pending = summary_of(&[]);
        pending.pending.push("0001_create_expenses".into());
        m.handle(Msg::Summary {
            project: "a".into(),
            result: Ok(pending),
        });
        assert!(m.can_apply());
        assert!(m.table_names().is_empty());

        m.begin_send();
        assert!(!m.can_apply());
        m.handle(Msg::Started {
            project: "a".into(),
            result: Ok("r1".into()),
        });
        assert!(!m.can_apply());
        m.handle(Msg::Run {
            project: "a".into(),
            run_id: "r1".into(),
            result: Ok(RunStatus {
                status: "finished".into(),
                error: None,
            }),
        });
        let mut now = Instant::now();
        let mut summary_requested = false;
        while let Some(poll) = m.next_poll(now) {
            summary_requested |= matches!(poll, Poll::Summary { .. });
            now += Duration::from_millis(1);
        }
        assert!(summary_requested);
        m.handle(Msg::Summary {
            project: "a".into(),
            result: Ok(summary_of(&[("expenses", 0)])),
        });
        assert!(!m.can_apply());
        assert_eq!(m.table_names(), ["expenses"]);
        assert!(matches!(m.next_poll(now), Some(Poll::Rows { .. })));
    }

    #[test]
    fn finished_run_reloads_questions_and_data() {
        let mut m = ready("a");
        m.handle(Msg::Started {
            project: "a".into(),
            result: Ok("r1".into()),
        });
        let fx = m.handle(Msg::Run {
            project: "a".into(),
            run_id: "r1".into(),
            result: Ok(RunStatus {
                status: "finished".into(),
                error: None,
            }),
        });
        assert_eq!(fx, vec![Effect::RefreshProjects]);
        let mut now = Instant::now() + Duration::from_secs(10);
        let mut kinds = Vec::new();
        while let Some(p) = m.next_poll(now) {
            kinds.push(
                matches!(p, Poll::Questions { .. }) as u8
                    + 2 * matches!(p, Poll::Summary { .. }) as u8,
            );
            now += Duration::from_millis(1);
        }
        assert!(kinds.contains(&1) && kinds.contains(&2), "{kinds:?}");
    }

    #[test]
    fn switching_tab_starts_at_the_first_page_and_hides_the_old_rows() {
        let mut m = ready("a");
        m.handle(Msg::Summary {
            project: "a".into(),
            result: Ok(summary_of(&[("expenses", 120), ("people", 1)])),
        });
        let now = Instant::now() + Duration::from_secs(10);
        m.next_poll(now);
        m.handle(Msg::Rows {
            project: "a".into(),
            table: "expenses".into(),
            offset: 0,
            result: Ok(rows_page("expenses", 0, 50, 120)),
        });
        m.choose_table("people");
        assert_eq!(m.table_index(), 1);
        assert!(m.current_page().is_none());
        assert_eq!(
            m.next_poll(now),
            Some(Poll::Rows {
                project: "a".into(),
                table: "people".into(),
                offset: 0
            })
        );
        // A late answer for the old tab is ignored.
        m.handle(Msg::Rows {
            project: "a".into(),
            table: "expenses".into(),
            offset: 0,
            result: Ok(rows_page("expenses", 0, 50, 120)),
        });
        assert!(m.current_page().is_none());
    }

    #[test]
    fn paging_moves_by_page_and_stops_at_the_ends() {
        let mut m = ready("a");
        m.handle(Msg::Summary {
            project: "a".into(),
            result: Ok(summary_of(&[("expenses", 120)])),
        });
        let now = Instant::now() + Duration::from_secs(10);
        let load = |m: &mut Model, offset: u64, count: u64| {
            m.handle(Msg::Rows {
                project: "a".into(),
                table: "expenses".into(),
                offset,
                result: Ok(rows_page("expenses", offset, count, 120)),
            });
        };
        assert!(matches!(
            m.next_poll(now),
            Some(Poll::Rows { offset: 0, .. })
        ));
        load(&mut m, 0, 50);
        assert_eq!(page_label(m.current_page().unwrap()), "Rows 1-50 of 120");
        assert!(!m.can_prev());
        assert!(m.can_next());

        m.next_page();
        assert_eq!(
            m.next_poll(now),
            Some(Poll::Rows {
                project: "a".into(),
                table: "expenses".into(),
                offset: 50
            })
        );
        load(&mut m, 50, 50);
        m.next_page();
        assert!(matches!(
            m.next_poll(now),
            Some(Poll::Rows { offset: 100, .. })
        ));
        load(&mut m, 100, 20);
        assert_eq!(page_label(m.current_page().unwrap()), "Rows 101-120 of 120");
        assert!(!m.can_next());
        m.next_page();
        assert_eq!(m.offset, 100, "no page past the end");
        m.prev_page();
        assert_eq!(m.offset, 50);
    }

    #[test]
    fn a_page_past_the_end_after_rows_were_removed_goes_to_the_last_page() {
        let mut m = ready("a");
        m.handle(Msg::Summary {
            project: "a".into(),
            result: Ok(summary_of(&[("expenses", 120)])),
        });
        m.offset = 100;
        let now = Instant::now() + Duration::from_secs(10);
        assert!(matches!(
            m.next_poll(now),
            Some(Poll::Rows { offset: 100, .. })
        ));
        m.handle(Msg::Rows {
            project: "a".into(),
            table: "expenses".into(),
            offset: 100,
            result: Ok(rows_page("expenses", 100, 0, 60)),
        });
        assert_eq!(m.offset, 50);
        assert!(matches!(
            m.next_poll(now),
            Some(Poll::Rows { offset: 50, .. })
        ));
        assert_eq!(last_page_offset(0, 50), 0);
        assert_eq!(last_page_offset(50, 50), 0);
        assert_eq!(last_page_offset(51, 50), 50);
    }

    #[test]
    fn table_that_disappears_falls_back_to_the_first_one() {
        let mut m = ready("a");
        m.handle(Msg::Summary {
            project: "a".into(),
            result: Ok(summary_of(&[("a_list", 1), ("b_list", 1)])),
        });
        m.choose_table("b_list");
        m.handle(Msg::Summary {
            project: "a".into(),
            result: Ok(summary_of(&[("a_list", 1)])),
        });
        assert_eq!(m.table.as_deref(), Some("a_list"));
        m.handle(Msg::Summary {
            project: "a".into(),
            result: Ok(summary_of(&[])),
        });
        assert_eq!(m.table, None);
        assert!(m.current_page().is_none());
    }

    #[test]
    fn retry_is_offered_after_a_failure_and_cleared_by_progress() {
        let mut m = ready("a");
        let failed = |seq| {
            ev(
                seq,
                EventKind::PromptFailed {
                    prompt_seq: 1,
                    error: "e".into(),
                    run_id: None,
                    detail: "Try again.".into(),
                },
            )
        };
        m.handle(Msg::Events {
            project: "a".into(),
            result: page(vec![
                ev(1, EventKind::UserPrompt { text: "hi".into() }),
                failed(2),
            ]),
        });
        assert!(m.retry, "also when the failure is in the history");
        m.handle(Msg::Started {
            project: "a".into(),
            result: Ok("r1".into()),
        });
        assert!(!m.retry, "a new run clears it");
        m.handle(Msg::Run {
            project: "a".into(),
            run_id: "r1".into(),
            result: Ok(RunStatus {
                status: "failed".into(),
                error: Some("No answer.".into()),
            }),
        });
        assert!(m.retry);
    }
}
