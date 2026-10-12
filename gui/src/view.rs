//! The single-column window: project dropdown, alert, prompt box with Send, status strip and
//! the "New project" modal. All logic lives in `Model`; this file only draws and forwards clicks.

use std::path::PathBuf;
use std::time::Instant;

use akar_components::{
    AKAR_THEME_DARK, AkarTheme, AlertVariant, ButtonVariant, DataGridAlign, DataGridColumn,
    DataGridSortDirection, DataGridState, DataGridStyle, TabVariant, TextEditState, akar_alert,
    akar_button, akar_tab_bar, akar_text_input, akar_textarea, data_grid_begin,
    data_grid_body_begin, data_grid_body_end, data_grid_cell, data_grid_end,
    data_grid_header_begin, data_grid_header_cell, data_grid_header_end, modal_begin, modal_end,
};
use akar_core::{AkarCore, Key, QuadCall, TextCall, Z_FLOAT, Z_OVERLAY};
use akar_layout::{
    AlignItems, Dimension, Display, FlexDirection, Layout, NodeId, Size, Style, length,
};
use nocodo_types::naming::words;
use nocodo_types::question::Question;
use nocodo_types::schema::ColumnType;

use crate::client::ApiClient;
use crate::grid::{cell_text, column_widths};
use crate::model::{Effect, Model, Poll, page_label};
use crate::settings::{self, Settings};

const THEME: AkarTheme = AKAR_THEME_DARK;
const MARGIN: f32 = 12.0;
const HEADER_H: f32 = 48.0;
const BTN_H: f32 = 32.0;
const ROW_H: f32 = 30.0;
const MAX_ROWS: usize = 7;
const ALERT_H: f32 = 64.0;
const PROMPT_H: f32 = 96.0;
const LINE_H: f32 = 22.0;
/// Status lines shown; the rest are in the event log.
const STATUS_LINES: usize = 3;
const CHOICE_H: f32 = 26.0;
const TAB_H: f32 = 34.0;
const PAGER_H: f32 = 36.0;
const SUMMARY_H: f32 = 66.0;
const MIN_GRID_H: f32 = 120.0;
const MODAL_NS: u64 = 0x00D1_A106;
const DATA_NS: u64 = 0x00D1_A107;
const CONFIRM_NS: u64 = 0x00D1_A108;

// Fixed text-buffer ids (the pipeline keys buffers by id).
const BUF_LABEL: u64 = 0xC0DE_0001;
const BUF_CHEVRON: u64 = 0xC0DE_0002;
const BUF_TITLE: u64 = 0xC0DE_0003;
const BUF_HINT: u64 = 0xC0DE_0004;
const BUF_ROW: u64 = 0xC0DE_1000;
const BUF_LINE: u64 = 0xC0DE_2000;
const BUF_Q: u64 = 0xC0DE_3000;
const BUF_BTN: u64 = 0xC0DE_4000;
const BUF_SUMMARY: u64 = 0xC0DE_4100;
const BUF_CONFIRM: u64 = 0xC0DE_5000;

struct NewProjectForm {
    name: String,
    edit: TextEditState,
}

pub struct View {
    client: ApiClient,
    model: Model,
    settings_path: Option<PathBuf>,
    layout: Layout,
    root: NodeId,
    header: NodeId,
    project_btn: NodeId,
    alert: NodeId,
    prompt: NodeId,
    send: NodeId,
    status: NodeId,
    popup_open: bool,
    popup_scroll: f32,
    prompt_text: String,
    prompt_edit: TextEditState,
    prompt_scroll: f32,
    new_project: Option<NewProjectForm>,
    q_scroll: f32,
    grid_state: DataGridState,
    /// The list and page the grid state belongs to; the scroll resets when it changes.
    grid_key: Option<(String, u64)>,
}

fn rgba(c: u32) -> [f32; 4] {
    [
        ((c >> 24) & 0xFF) as f32 / 255.0,
        ((c >> 16) & 0xFF) as f32 / 255.0,
        ((c >> 8) & 0xFF) as f32 / 255.0,
        (c & 0xFF) as f32 / 255.0,
    ]
}

fn quad(rect: [f32; 4], fill: u32, radius: f32) -> QuadCall {
    QuadCall {
        rect,
        fill: rgba(fill),
        border_color: [0.0; 4],
        corner_radii: [radius; 4],
        border_width: 0.0,
        z: 0.0,
        shadow_blur: 0.0,
        shadow_spread: 0.0,
        shadow_color: [0.0; 4],
        shadow_offset: [0.0; 2],
        _pad: [0.0; 2],
    }
}

fn clip_to(r: [f32; 4], bounds: [f32; 4]) -> [f32; 4] {
    let x0 = r[0].max(bounds[0]);
    let y0 = r[1].max(bounds[1]);
    let x1 = (r[0] + r[2]).min(bounds[0] + bounds[2]);
    let y1 = (r[1] + r[3]).min(bounds[1] + bounds[3]);
    [x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0)]
}

/// Height of `text` wrapped to `width`.
fn wrapped_h(core: &mut AkarCore, id: u64, text: &str, size: f32, width: f32) -> f32 {
    let buf = core.text_pipeline.set_text(
        Some(id),
        text,
        glyphon::Metrics::new(size, size * 1.2),
        Some(width),
        None,
        None,
    );
    core.text_pipeline.measure(buf, Some(width)).y
}

/// A plain button; true when clicked. `z` is 0 on the page and `Z_FLOAT` on a modal.
fn button(
    core: &mut AkarCore,
    id: u64,
    r: [f32; 4],
    label: &str,
    primary: bool,
    enabled: bool,
    z: f32,
) -> bool {
    let hovered = enabled && core.input.is_hovering(r);
    let fill = if !enabled {
        THEME.base_300
    } else if primary {
        THEME.primary
    } else if hovered {
        THEME.base_100
    } else {
        THEME.base_300
    };
    let quad_z = if z > 0.0 { z + 0.01 } else { 0.0 };
    core.draw_list.push_quad(QuadCall {
        z: quad_z,
        ..quad(r, fill, THEME.radius_field)
    });
    let colour = if !enabled {
        THEME.muted_content
    } else if primary {
        THEME.primary_content
    } else {
        THEME.base_content
    };
    let metrics = glyphon::Metrics::new(THEME.font_size_base, THEME.font_size_base * 1.2);
    let buf = core
        .text_pipeline
        .set_text(Some(id), label, metrics, None, None, None);
    let tw = core.text_pipeline.measure(buf, None).x;
    core.draw_list.push_text(TextCall {
        buffer_id: buf,
        x: r[0] + (r[2] - tw) / 2.0,
        y: r[1] + (r[3] - metrics.line_height) / 2.0,
        clip: r,
        color: rgba(colour),
        z,
    });
    enabled && core.input.is_clicked(r)
}

fn leaf(layout: &mut Layout, width: Dimension, height: f32, grow: f32) -> NodeId {
    layout.new_leaf(Style {
        flex_grow: grow,
        size: Size {
            width,
            height: length(height),
        },
        ..Default::default()
    })
}

#[allow(clippy::too_many_arguments)]
fn draw_text(
    core: &mut AkarCore,
    id: u64,
    text: &str,
    size: f32,
    pos: [f32; 2],
    clip: [f32; 4],
    color: u32,
    z: f32,
) {
    let buf = core.text_pipeline.set_text(
        Some(id),
        text,
        glyphon::Metrics::new(size, size * 1.2),
        Some(clip[2]),
        None,
        None,
    );
    core.draw_list.push_text(TextCall {
        buffer_id: buf,
        x: pos[0],
        y: pos[1],
        clip,
        color: rgba(color),
        z,
    });
}

impl View {
    pub fn new(client: ApiClient) -> Self {
        let settings_path = settings::default_path();
        let remembered = settings_path
            .as_deref()
            .and_then(|p| settings::load(p).project);
        client.list_projects();

        let mut layout = Layout::new();
        let project_btn = leaf(&mut layout, Dimension::auto(), BTN_H, 1.0);
        let header = layout.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::CENTER),
                size: Size {
                    width: Dimension::percent(1.0),
                    height: length(HEADER_H),
                },
                ..Default::default()
            },
            &[project_btn],
        );
        let alert = leaf(&mut layout, Dimension::percent(1.0), 0.0, 0.0);
        let prompt = leaf(&mut layout, Dimension::auto(), PROMPT_H, 1.0);
        layout.set_margin(prompt, 0.0, 8.0, 0.0, 0.0);
        let send = leaf(&mut layout, length(72.0_f32), BTN_H, 0.0);
        let prompt_bar = layout.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::FLEX_START),
                size: Size {
                    width: Dimension::percent(1.0),
                    height: length(PROMPT_H),
                },
                ..Default::default()
            },
            &[prompt, send],
        );
        let status = layout.new_leaf(Style {
            flex_grow: 1.0,
            size: Size {
                width: Dimension::percent(1.0),
                height: Dimension::auto(),
            },
            ..Default::default()
        });
        layout.set_margin(status, 12.0, 0.0, 0.0, 0.0);
        let root = layout.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            },
            &[header, alert, prompt_bar, status],
        );
        layout.set_padding(root, 0.0, MARGIN, MARGIN, MARGIN);

        Self {
            client,
            model: Model::new(remembered),
            settings_path,
            layout,
            root,
            header,
            project_btn,
            alert,
            prompt,
            send,
            status,
            popup_open: false,
            popup_scroll: 0.0,
            prompt_text: String::new(),
            prompt_edit: TextEditState::default(),
            prompt_scroll: 0.0,
            new_project: None,
            q_scroll: 0.0,
            grid_state: DataGridState::new(),
            grid_key: None,
        }
    }

    fn apply_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Remember(folder) => {
                    if let Some(path) = &self.settings_path {
                        // Remembering is a convenience; a failed save is not worth an alert.
                        let _ = settings::save(
                            path,
                            &Settings {
                                project: Some(folder),
                            },
                        );
                    }
                }
                Effect::RefreshProjects => self.client.list_projects(),
            }
        }
    }

    /// Takes finished requests and starts the polls that are due.
    fn pump(&mut self) {
        while let Some(msg) = self.client.try_recv() {
            let effects = self.model.handle(msg);
            self.apply_effects(effects);
        }
        match self.model.next_poll(Instant::now()) {
            Some(Poll::Events { project, after }) => self.client.events(project, after),
            Some(Poll::Run { project, run_id }) => self.client.run_status(project, run_id),
            Some(Poll::Questions { project }) => self.client.questions(project),
            Some(Poll::Summary { project }) => self.client.summary(project),
            Some(Poll::Rows {
                project,
                table,
                offset,
            }) => self
                .client
                .rows(project, table, offset, crate::model::PAGE_SIZE),
            None => {}
        }
    }

    fn choose(&mut self, folder: &str) {
        let effects = self.model.select(folder);
        self.apply_effects(effects);
        self.popup_open = false;
    }

    fn submit(&mut self) {
        let text = self.prompt_text.trim().to_string();
        if text.is_empty() || !self.model.can_send() {
            return;
        }
        if let Some(project) = self.model.selected.clone() {
            self.model.begin_send();
            self.client.send_prompt(project, text);
            self.prompt_text.clear();
            self.prompt_edit = TextEditState::default();
            self.prompt_scroll = 0.0;
        }
    }

    /// Draws one frame. `size` is the logical window size.
    pub fn render(&mut self, core: &mut AkarCore, size: [f32; 2], cursor_visible: bool) {
        self.pump();

        let alert_h = if self.model.error.is_some() {
            ALERT_H
        } else {
            0.0
        };
        self.layout.set_style(
            self.alert,
            Style {
                size: Size {
                    width: Dimension::percent(1.0),
                    height: length(alert_h),
                },
                ..Default::default()
            },
        );
        self.layout.compute(
            self.root,
            (Some(size[0]), Some(size[1])),
            |_, _, _, _, _| Size::ZERO,
        );

        core.draw_list
            .push_quad(quad([0.0, 0.0, size[0], size[1]], THEME.base_100, 0.0));

        // akar draws all text above all quads, so an overlay replaces the content under it.
        if self.new_project.is_some() {
            self.draw_new_project_modal(core, size, cursor_visible);
            return;
        }
        if self.model.confirmation().is_some() && self.model.can_send() {
            self.draw_confirm_modal(core, size);
            return;
        }
        self.draw_header(core);
        if self.popup_open {
            return;
        }
        self.draw_alert(core);
        self.draw_prompt(core, cursor_visible);
        self.draw_lower(core);
    }

    fn draw_header(&mut self, core: &mut AkarCore) {
        let header = self.layout.rect(self.header);
        let btn = self.layout.rect(self.project_btn);
        core.draw_list.push_quad(quad(
            [header[0], header[1] + header[3] - 1.0, header[2], 1.0],
            THEME.base_300,
            0.0,
        ));
        let hovered = core.input.is_hovering(btn);
        core.draw_list.push_quad(QuadCall {
            border_color: rgba(THEME.base_300),
            border_width: THEME.border_width,
            ..quad(
                btn,
                if hovered {
                    THEME.base_300
                } else {
                    THEME.base_200
                },
                THEME.radius_field,
            )
        });
        let label = self.model.project_label().to_string();
        let colour = if self.model.selected.is_some() {
            THEME.base_content
        } else {
            THEME.muted_content
        };
        draw_text(
            core,
            BUF_LABEL,
            &label,
            THEME.font_size_base,
            [
                btn[0] + THEME.padding_x,
                btn[1] + (btn[3] - THEME.font_size_base * 1.2) / 2.0,
            ],
            [btn[0], btn[1], btn[2] - 28.0, btn[3]],
            colour,
            0.0,
        );
        draw_text(
            core,
            BUF_CHEVRON,
            "\u{25BC}",
            THEME.font_size_sm,
            [
                btn[0] + btn[2] - 20.0,
                btn[1] + (btn[3] - THEME.font_size_sm * 1.2) / 2.0,
            ],
            btn,
            THEME.base_content,
            0.0,
        );

        // Rows: the projects, then "New project…" at the end.
        let total_rows = self.model.projects.len() + 1;
        let popup = [
            btn[0],
            btn[1] + btn[3] + 4.0,
            btn[2],
            total_rows.min(MAX_ROWS) as f32 * ROW_H,
        ];
        let toggled = core.input.is_clicked(btn);
        if self.popup_open
            && core.input.mouse_buttons_pressed[0]
            && !core.input.is_hovering(btn)
            && !core.input.is_hovering(popup)
        {
            self.popup_open = false;
        }
        if toggled {
            self.popup_open = !self.popup_open;
            self.popup_scroll = 0.0;
        }
        if !self.popup_open {
            return;
        }
        let max_scroll = (total_rows as f32 * ROW_H - popup[3]).max(0.0);
        if core.input.is_hovering(popup) {
            self.popup_scroll -= core.input.scroll_delta.y;
            core.input.scroll_delta.y = 0.0;
        }
        self.popup_scroll = self.popup_scroll.clamp(0.0, max_scroll);

        core.draw_list.push_quad(QuadCall {
            border_color: rgba(THEME.base_300),
            border_width: 1.0,
            shadow_blur: 12.0,
            shadow_color: [0.0, 0.0, 0.0, 0.25],
            z: Z_OVERLAY,
            ..quad(popup, THEME.base_100, THEME.radius_field)
        });
        let inner = [
            popup[0] + 1.0,
            popup[1] + 1.0,
            popup[2] - 2.0,
            popup[3] - 2.0,
        ];
        let mut chosen = None;
        for i in 0..total_rows {
            let row = [
                inner[0],
                inner[1] + i as f32 * ROW_H - self.popup_scroll,
                inner[2],
                ROW_H,
            ];
            if row[1] + row[3] < inner[1] || row[1] > inner[1] + inner[3] {
                continue;
            }
            let visible_row = clip_to(row, inner);
            let (text, usable, active) = match self.model.projects.get(i) {
                Some(p) if p.is_usable() => (
                    p.id().to_string(),
                    true,
                    self.model.selected.as_deref() == Some(p.id()),
                ),
                Some(p) => (format!("{} (cannot be opened)", p.name), false, false),
                None => ("New project\u{2026}".to_string(), true, false),
            };
            if usable && core.input.is_hovering(visible_row) {
                core.draw_list.push_quad(QuadCall {
                    z: Z_OVERLAY,
                    ..quad(visible_row, THEME.base_300, 0.0)
                });
                if core.input.is_clicked(visible_row) {
                    chosen = Some(i);
                }
            }
            let colour = if !usable {
                THEME.muted_content
            } else if active {
                THEME.primary
            } else {
                THEME.base_content
            };
            draw_text(
                core,
                BUF_ROW + i as u64,
                &text,
                THEME.font_size_base,
                [
                    row[0] + THEME.padding_x,
                    row[1] + (ROW_H - THEME.font_size_base * 1.2) / 2.0,
                ],
                visible_row,
                colour,
                Z_OVERLAY,
            );
        }
        if let Some(i) = chosen {
            self.popup_open = false;
            match self.model.projects.get(i).map(|p| p.id().to_string()) {
                Some(folder) => self.choose(&folder),
                None => {
                    self.new_project = Some(NewProjectForm {
                        name: String::new(),
                        edit: TextEditState::default(),
                    });
                    core.input.focused_id = None;
                }
            }
        }
    }

    fn draw_alert(&mut self, core: &mut AkarCore) {
        let Some(text) = self.model.error.clone() else {
            return;
        };
        if akar_alert(
            core,
            &self.layout,
            self.alert,
            &text,
            AlertVariant::Error,
            true,
            &THEME,
        )
        .dismissed
        {
            self.model.error = None;
        }
    }

    fn draw_prompt(&mut self, core: &mut AkarCore, cursor_visible: bool) {
        let placeholder = if self.model.selected.is_some() {
            "Say in your own words what you want to keep track of"
        } else {
            "Pick a project above first"
        };
        akar_textarea(
            core,
            &self.layout,
            self.prompt,
            &mut self.prompt_text,
            &mut self.prompt_edit,
            &mut self.prompt_scroll,
            placeholder,
            cursor_visible,
            &THEME,
        );
        let label = if self.model.is_running() {
            "Working"
        } else {
            "Send"
        };
        let send = akar_button(
            core,
            &self.layout,
            self.send,
            label,
            ButtonVariant::Solid,
            &THEME,
        );
        if send.clicked {
            self.submit();
        }
    }

    /// Starts a run with a request that has no text of its own (answers, retry, sample rows).
    fn start(&mut self, route: &'static str, body: serde_json::Value) {
        if let Some(project) = self.model.selected.clone() {
            self.model.begin_send();
            self.client.start(project, route, body);
        }
    }

    /// Everything under the prompt box: what happened, the questions, the summary and the data.
    fn draw_lower(&mut self, core: &mut AkarCore) {
        let area = self.layout.rect(self.status);
        let (x, w) = (area[0], area[2]);
        let bottom = area[1] + area[3];
        let mut y = area[1];
        let title = if self.model.is_running() {
            "Working on it..."
        } else {
            "What happened"
        };
        draw_text(
            core,
            BUF_TITLE,
            title,
            THEME.font_size_sm,
            [x, y],
            [x, y, (w - 100.0).max(0.0), 20.0],
            THEME.muted_content,
            0.0,
        );
        if self.model.retry
            && self.model.can_send()
            && button(
                core,
                BUF_BTN,
                [x + w - 84.0, y - 4.0, 84.0, 28.0],
                "Retry",
                true,
                true,
                0.0,
            )
        {
            self.start("/continue", serde_json::json!({}));
        }
        y += 26.0;
        if self.model.selected.is_some() && self.model.lines.is_empty() {
            draw_text(
                core,
                BUF_HINT,
                "Nothing yet. Describe your first list above and press Send.",
                THEME.font_size_base,
                [x, y],
                [x, y, w, LINE_H],
                THEME.muted_content,
                0.0,
            );
            y += LINE_H;
        }
        let skip = self.model.lines.len().saturating_sub(STATUS_LINES);
        for (i, line) in self.model.lines.iter().skip(skip).enumerate() {
            draw_text(
                core,
                BUF_LINE + i as u64,
                line,
                THEME.font_size_base,
                [x, y],
                [x, y, w, LINE_H],
                THEME.base_content,
                0.0,
            );
            y += LINE_H;
        }
        y += 10.0;
        if self.model.selected.is_none() {
            return;
        }
        y = self.draw_questions(core, x, y, w, bottom);
        self.draw_data(core, x, y, w, bottom);
    }

    /// The assistant's open questions as numbered choices. Returns the y below the panel.
    fn draw_questions(&mut self, core: &mut AkarCore, x: f32, y: f32, w: f32, bottom: f32) -> f32 {
        let qs: Vec<Question> = self.model.open_questions().into_iter().cloned().collect();
        if qs.is_empty() {
            return y;
        }
        let pad = 10.0;
        let footer_h = 44.0;
        let inner_w = (w - 2.0 * pad - 12.0).max(40.0);
        // Heights first: the text wraps.
        let mut text_hs = Vec::new();
        let mut content_h = 0.0;
        for (i, q) in qs.iter().enumerate() {
            let h = wrapped_h(
                core,
                BUF_Q + i as u64 * 64,
                &question_text(q),
                THEME.font_size_base,
                inner_w,
            );
            text_hs.push(h);
            content_h += h + 6.0 + q.choices.len() as f32 * CHOICE_H + 12.0;
        }
        let max_h = ((bottom - y) * 0.55).max(180.0);
        let region_h = content_h.min(max_h - footer_h - 2.0 * pad).max(60.0);
        let panel = [x, y, w, region_h + footer_h + 2.0 * pad];
        let region = [x + pad, y + pad, w - 2.0 * pad, region_h];
        core.draw_list
            .push_quad(quad(panel, THEME.base_200, THEME.radius_box));

        if core.input.is_hovering(region) {
            self.q_scroll -= core.input.scroll_delta.y;
            core.input.scroll_delta.y = 0.0;
        }
        self.q_scroll = self.q_scroll.clamp(0.0, (content_h - region_h).max(0.0));

        let mut picked = None;
        core.draw_list.push_scissor(region);
        let mut cy = region[1] - self.q_scroll;
        for (i, q) in qs.iter().enumerate() {
            let base = BUF_Q + i as u64 * 64;
            let text_h = text_hs[i];
            draw_text(
                core,
                base,
                &question_text(q),
                THEME.font_size_base,
                [region[0], cy],
                clip_to([region[0], cy, inner_w, text_h], region),
                if q.blocking {
                    THEME.warning
                } else {
                    THEME.base_content
                },
                0.0,
            );
            cy += text_h + 6.0;
            for (j, choice) in q.choices.iter().enumerate().take(63) {
                let n = j + 1;
                let row = [region[0], cy, region[2], CHOICE_H];
                cy += CHOICE_H;
                let shown = clip_to(row, region);
                if shown[3] <= 0.0 {
                    continue;
                }
                let chosen = self.model.picks.get(&q.id) == Some(&n);
                if chosen || core.input.is_hovering(shown) {
                    core.draw_list.push_quad(quad(
                        shown,
                        if chosen {
                            THEME.base_300
                        } else {
                            THEME.base_100
                        },
                        THEME.radius_field,
                    ));
                }
                if core.input.is_clicked(shown) {
                    picked = Some((q.id.clone(), n));
                }
                draw_text(
                    core,
                    base + 1 + j as u64,
                    &choice_text(n, choice, chosen, q.default == Some(n)),
                    THEME.font_size_base,
                    [
                        row[0] + 8.0,
                        row[1] + (CHOICE_H - THEME.font_size_base * 1.2) / 2.0,
                    ],
                    clip_to([row[0] + 8.0, row[1], row[2] - 8.0, CHOICE_H], region),
                    THEME.base_content,
                    0.0,
                );
            }
            cy += 12.0;
        }
        core.draw_list.pop_scissor();
        if let Some((id, n)) = picked {
            self.model.pick(&id, n);
        }

        let fy = y + panel[3] - footer_h + 6.0;
        let enabled = self.model.can_send();
        let defaults = button(
            core,
            BUF_BTN + 1,
            [x + pad, fy, 130.0, 32.0],
            "Use defaults",
            false,
            enabled && self.model.can_use_defaults(),
            0.0,
        );
        let answer = button(
            core,
            BUF_BTN + 2,
            [x + pad + 140.0, fy, 100.0, 32.0],
            "Answer",
            true,
            enabled && !self.model.picks.is_empty(),
            0.0,
        );
        if defaults {
            self.start("/answers", serde_json::json!({ "defaults": true }));
        } else if answer && let Some(body) = self.model.take_answers() {
            self.start("/answers", body);
        }
        y + panel[3] + 10.0
    }

    /// The plain-language summary, a tab per list and the rows of the chosen list.
    fn draw_data(&mut self, core: &mut AkarCore, x: f32, y: f32, w: f32, bottom: f32) {
        let Some(summary) = self.model.summary.clone() else {
            return;
        };
        let mut y = y;
        if !summary.summary.trim().is_empty() {
            draw_text(
                core,
                BUF_SUMMARY,
                summary.summary.trim(),
                THEME.font_size_base,
                [x, y],
                [x, y, w, SUMMARY_H],
                THEME.base_content,
                0.0,
            );
            y += SUMMARY_H + 8.0;
        }
        let names = self.model.table_names();
        if names.is_empty() {
            if bottom - y > 30.0 {
                draw_text(
                    core,
                    BUF_SUMMARY + 1,
                    "No lists yet. Your lists appear here once your first request is done.",
                    THEME.font_size_base,
                    [x, y],
                    [x, y, w, 24.0],
                    THEME.muted_content,
                    0.0,
                );
            }
            return;
        }
        if bottom - y < TAB_H + PAGER_H + MIN_GRID_H {
            return;
        }

        // Tabs, grid and pager in a layout of their own, placed at (x, y).
        let mut tl = Layout::new();
        tl.set_namespace_id(DATA_NS);
        let tabs = leaf(&mut tl, length(w), TAB_H, 0.0);
        let grid = leaf(&mut tl, length(w), MIN_GRID_H, 1.0);
        let pager = leaf(&mut tl, length(w), PAGER_H, 0.0);
        let root = tl.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: length(x + w),
                    height: length(bottom),
                },
                ..Default::default()
            },
            &[tabs, grid, pager],
        );
        tl.set_padding(root, y, 0.0, 0.0, x);
        tl.compute(root, (Some(x + w), Some(bottom)), |_, _, _, _, _| {
            Size::ZERO
        });

        let labels: Vec<String> = names.iter().map(|n| words(n)).collect();
        let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let clicked = akar_tab_bar(
            core,
            &tl,
            tabs,
            &label_refs,
            self.model.table_index(),
            TabVariant::Underline,
            &THEME,
        )
        .clicked;
        if let Some(i) = clicked
            && let Some(name) = names.get(i)
        {
            self.model.choose_table(name);
        }

        let page = self.model.current_page().cloned();
        let grid_rect = tl.rect(grid);
        match page {
            Some(page) => {
                let key = (page.table.clone(), page.offset);
                if self.grid_key.as_ref() != Some(&key) {
                    self.grid_state = DataGridState::new();
                    self.grid_key = Some(key);
                }
                self.draw_grid(core, &tl, grid, &page);
            }
            None => draw_text(
                core,
                BUF_SUMMARY + 2,
                "Loading...",
                THEME.font_size_base,
                [grid_rect[0] + 8.0, grid_rect[1] + 8.0],
                grid_rect,
                THEME.muted_content,
                0.0,
            ),
        }

        let pr = tl.rect(pager);
        let by = pr[1] + 2.0;
        let can_prev = self.model.can_prev();
        let can_next = self.model.can_next();
        if button(
            core,
            BUF_BTN + 3,
            [pr[0], by, 80.0, 32.0],
            "Previous",
            false,
            can_prev,
            0.0,
        ) {
            self.model.prev_page();
        }
        if button(
            core,
            BUF_BTN + 4,
            [pr[0] + 88.0, by, 80.0, 32.0],
            "Next",
            false,
            can_next,
            0.0,
        ) {
            self.model.next_page();
        }
        let label = self
            .model
            .current_page()
            .map(page_label)
            .unwrap_or_default();
        draw_text(
            core,
            BUF_BTN + 5,
            &label,
            THEME.font_size_base,
            [pr[0] + 180.0, by + 6.0],
            [pr[0] + 180.0, by, (pr[2] - 180.0 - 170.0).max(0.0), 32.0],
            THEME.muted_content,
            0.0,
        );
        if button(
            core,
            BUF_BTN + 6,
            [pr[0] + pr[2] - 160.0, by, 160.0, 32.0],
            "Add sample rows",
            false,
            self.model.can_send(),
            0.0,
        ) {
            self.start("/sample-rows", serde_json::json!({}));
        }
    }

    fn draw_grid(
        &mut self,
        core: &mut AkarCore,
        tl: &Layout,
        node: NodeId,
        page: &nocodo_types::outcome::RowPage,
    ) {
        let style = DataGridStyle::from_theme(&THEME);
        let widths = column_widths(&page.columns, &page.rows);
        let columns: Vec<DataGridColumn> = page
            .columns
            .iter()
            .zip(&widths)
            .enumerate()
            .map(|(i, (c, w))| DataGridColumn {
                key: i as u64 + 1,
                width: *w,
                align: match c.column_type {
                    ColumnType::Integer | ColumnType::Money | ColumnType::Decimal => {
                        DataGridAlign::Right
                    }
                    _ => DataGridAlign::Left,
                },
            })
            .collect();
        let row_keys: Vec<u64> = (0..page.rows.len())
            .map(|i| page.offset + i as u64 + 1)
            .collect();
        let resp = data_grid_begin(
            core,
            tl,
            node,
            &mut self.grid_state,
            page.rows.len(),
            &row_keys,
            style.row_height,
            style.header_height,
            &columns,
            &style,
        );
        data_grid_header_begin(core, &resp, &style);
        for c in resp.visible_columns.clone() {
            data_grid_header_cell(
                core,
                tl,
                &resp,
                node,
                c,
                &columns,
                &style,
                &page.columns[c].label,
                DataGridSortDirection::None,
            );
        }
        data_grid_header_end(core);
        data_grid_body_begin(core, &resp, &row_keys, &style, &[]);
        for r in resp.visible_rows.clone() {
            for c in resp.visible_columns.clone() {
                let text = page.rows[r]
                    .get(c)
                    .map(|v| cell_text(page.columns[c].column_type, v))
                    .unwrap_or_default();
                data_grid_cell(
                    core,
                    tl,
                    &resp,
                    node,
                    r,
                    row_keys[r],
                    c,
                    &columns,
                    &style,
                    &text,
                    false,
                );
            }
        }
        data_grid_body_end(core);
        data_grid_end(core);
    }

    /// The removal confirmation: the question in domain words and one button per choice.
    fn draw_confirm_modal(&mut self, core: &mut AkarCore, size: [f32; 2]) {
        let Some(q) = self.model.confirmation().cloned() else {
            return;
        };
        let width = size[0].min(460.0);
        let text_w = width - 32.0;
        let text_h = wrapped_h(core, BUF_CONFIRM, &q.text, THEME.font_size_base, text_w);
        let height = 56.0 + text_h + 16.0 + q.choices.len() as f32 * 40.0 + 16.0;
        let mut modal_layout = Layout::new();
        modal_layout.set_namespace_id(CONFIRM_NS);
        let m = modal_begin(
            core,
            &mut modal_layout,
            [0.0, 0.0, size[0], size[1]],
            "Please confirm",
            width,
            height,
            &THEME,
        );
        let [cx, cy, _, _] = m.content_rect;
        let mut y = cy + 8.0;
        draw_text(
            core,
            BUF_CONFIRM,
            &q.text,
            THEME.font_size_base,
            [cx + 16.0, y],
            [cx + 16.0, y, text_w, text_h],
            THEME.base_content,
            Z_FLOAT,
        );
        y += text_h + 16.0;
        let mut chosen = None;
        for (j, choice) in q.choices.iter().enumerate() {
            let n = j + 1;
            let label = if q.default == Some(n) {
                format!("{n}. {choice}  (suggested)")
            } else {
                format!("{n}. {choice}")
            };
            if button(
                core,
                BUF_CONFIRM + 1 + j as u64,
                [cx + 16.0, y, text_w, 32.0],
                &label,
                q.default == Some(n),
                true,
                Z_FLOAT,
            ) {
                chosen = Some(n);
            }
            y += 40.0;
        }
        modal_end(core);
        // Closing the panel does nothing: the choices are the only way out.
        if let Some(n) = chosen
            && let Some(body) = self.model.confirmation_answer(n)
        {
            self.start("/answers", body);
        }
    }

    /// Modal asking for the new project's name. Enter or Create submits; Esc, Cancel, the close
    /// button or a click outside the panel cancels.
    fn draw_new_project_modal(
        &mut self,
        core: &mut AkarCore,
        size: [f32; 2],
        cursor_visible: bool,
    ) {
        let viewport = [0.0, 0.0, size[0], size[1]];
        let mut modal_layout = Layout::new();
        modal_layout.set_namespace_id(MODAL_NS);
        let m = modal_begin(
            core,
            &mut modal_layout,
            viewport,
            "New project",
            320.0,
            150.0,
            &THEME,
        );
        let mut cancel = m.close_requested && {
            let close_btn = [
                m.panel_rect[0] + m.panel_rect[2] - 40.0,
                m.panel_rect[1],
                40.0,
                40.0,
            ];
            !core.input.is_hovering(m.panel_rect) || core.input.is_hovering(close_btn)
        };
        let escape = core.input.keys_pressed.contains(&Key::Escape);

        let fw = m.content_rect[2] - 32.0;
        let mut fl = Layout::new();
        fl.set_namespace_id(MODAL_NS + 1);
        let field = leaf(&mut fl, length(fw), 32.0, 0.0);
        fl.set_margin(field, 0.0, 0.0, 12.0, 0.0);
        let spacer = leaf(&mut fl, Dimension::auto(), 32.0, 1.0);
        let cancel_node = leaf(&mut fl, length(80.0_f32), 32.0, 0.0);
        fl.set_margin(cancel_node, 0.0, 8.0, 0.0, 0.0);
        let create_node = leaf(&mut fl, length(80.0_f32), 32.0, 0.0);
        let row = fl.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: length(fw),
                    height: length(32.0_f32),
                },
                ..Default::default()
            },
            &[spacer, cancel_node, create_node],
        );
        let root = fl.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            },
            &[field, row],
        );
        fl.set_padding(
            root,
            m.content_rect[1] + 8.0,
            0.0,
            0.0,
            m.content_rect[0] + 16.0,
        );
        fl.compute(root, (Some(size[0]), Some(size[1])), |_, _, _, _, _| {
            Size::ZERO
        });

        // akar's input and button quads sit at z=0, behind the modal panel, so the field
        // background and the buttons are drawn here, above the panel.
        let field_rect = fl.rect(field);
        core.draw_list.push_quad(QuadCall {
            border_color: rgba(THEME.primary),
            border_width: THEME.border_width,
            z: Z_FLOAT + 0.01,
            ..quad(field_rect, THEME.base_100, THEME.radius_field)
        });
        let mut create = false;
        for (node, label, is_create) in [
            (cancel_node, "Cancel", false),
            (create_node, "Create", true),
        ] {
            let r = fl.rect(node);
            let hovered = core.input.is_hovering(r);
            let fill = match (is_create, hovered) {
                (true, _) => THEME.primary,
                (false, false) => THEME.base_300,
                (false, true) => THEME.base_100,
            };
            core.draw_list.push_quad(QuadCall {
                z: Z_FLOAT + 0.01,
                ..quad(r, fill, THEME.radius_field)
            });
            let colour = if is_create {
                THEME.primary_content
            } else {
                THEME.base_content
            };
            let metrics = glyphon::Metrics::new(THEME.font_size_base, THEME.font_size_base * 1.2);
            let buf = core.text_pipeline.set_text(
                Some(fl.widget_id(node)),
                label,
                metrics,
                None,
                None,
                None,
            );
            let tw = core.text_pipeline.measure(buf, None).x;
            core.draw_list.push_text(TextCall {
                buffer_id: buf,
                x: r[0] + (r[2] - tw) / 2.0,
                y: r[1] + (r[3] - metrics.line_height) / 2.0,
                clip: r,
                color: rgba(colour),
                z: Z_FLOAT,
            });
            if core.input.is_clicked(r) {
                if is_create {
                    create = true;
                } else {
                    cancel = true;
                }
            }
        }

        let Some(form) = self.new_project.as_mut() else {
            modal_end(core);
            return;
        };
        let field_id = fl.widget_id(field);
        if core.input.focused_id.is_none() {
            core.input.focused_id = Some(field_id);
        }
        let input = akar_text_input(
            core,
            &fl,
            field,
            &mut form.name,
            &mut form.edit,
            "Project name",
            cursor_visible,
            &THEME,
        );
        modal_end(core);

        if escape || cancel {
            self.new_project = None;
            core.input.focused_id = None;
        } else if (input.submitted || create) && !form.name.trim().is_empty() {
            let name = form.name.trim().to_string();
            self.new_project = None;
            core.input.focused_id = None;
            self.client.create_project(name);
        }
    }
}

/// The question as shown: blocking ones say they need an answer.
fn question_text(q: &Question) -> String {
    if q.blocking {
        format!("Needs your answer: {}", q.text)
    } else {
        q.text.clone()
    }
}

/// One numbered choice line, with the marker and the suggested default.
fn choice_text(n: usize, choice: &str, chosen: bool, default: bool) -> String {
    let marker = if chosen { "\u{25CF}" } else { "\u{25CB}" };
    let suggested = if default { "  (suggested)" } else { "" };
    format!("{marker}  {n}. {choice}{suggested}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(blocking: bool) -> Question {
        Question {
            id: "q1".into(),
            asked_by: "schema".into(),
            text: "Weekly or monthly?".into(),
            choices: vec!["Weekly".into(), "Monthly".into()],
            default: if blocking { None } else { Some(2) },
            blocking,
            why: String::new(),
        }
    }

    #[test]
    fn blocking_questions_say_so() {
        assert_eq!(question_text(&q(false)), "Weekly or monthly?");
        assert_eq!(
            question_text(&q(true)),
            "Needs your answer: Weekly or monthly?"
        );
    }

    #[test]
    fn choices_are_numbered_from_one_and_the_default_is_marked() {
        assert_eq!(
            choice_text(2, "Monthly", false, true),
            "\u{25CB}  2. Monthly  (suggested)"
        );
        assert_eq!(choice_text(1, "Weekly", true, false), "\u{25CF}  1. Weekly");
    }
}
