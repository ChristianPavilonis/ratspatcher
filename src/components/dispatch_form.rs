use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::action::Action;
use crate::components::Component;
use crate::model::workflow::{InputType, Workflow, WorkflowInput};

/// Holds the current value for a form field.
#[derive(Debug)]
pub enum FieldValue {
    Text(Input),
    Boolean(bool),
    Choice {
        selected: usize,
        options: Vec<String>,
    },
}

/// A single field in the dispatch form.
pub struct FormField {
    pub input: WorkflowInput,
    pub value: FieldValue,
}

pub struct DispatchForm {
    pub visible: bool,
    pub workflow: Option<Workflow>,
    pub workflow_index: usize,
    pub fields: Vec<FormField>,
    pub focused_field: usize,
    pub ref_input: Input,
    pub ref_focused: bool,
    pub button_focused: bool,
    pub error_msg: Option<String>,
    pub dispatching: bool,
}

impl DispatchForm {
    pub fn new() -> Self {
        Self {
            visible: false,
            workflow: None,
            workflow_index: 0,
            fields: Vec::new(),
            focused_field: 0,
            ref_input: Input::default().with_value("main".to_string()),
            ref_focused: true,
            button_focused: false,
            error_msg: None,
            dispatching: false,
        }
    }

    pub fn open(&mut self, workflow: Workflow, index: usize, default_branch: &str) {
        self.fields = workflow
            .inputs
            .iter()
            .map(|input| {
                let value = match input.input_type {
                    InputType::Boolean => {
                        let default_val = input
                            .default
                            .as_deref()
                            .map(|d| d == "true")
                            .unwrap_or(false);
                        FieldValue::Boolean(default_val)
                    }
                    InputType::Choice => {
                        let default_idx = input
                            .default
                            .as_ref()
                            .and_then(|d| input.options.iter().position(|o| o == d))
                            .unwrap_or(0);
                        FieldValue::Choice {
                            selected: default_idx,
                            options: input.options.clone(),
                        }
                    }
                    _ => {
                        let default_val = input.default.clone().unwrap_or_default();
                        FieldValue::Text(Input::default().with_value(default_val))
                    }
                };
                FormField {
                    input: input.clone(),
                    value,
                }
            })
            .collect();

        self.workflow = Some(workflow);
        self.workflow_index = index;
        self.focused_field = 0;
        self.ref_input = Input::default().with_value(default_branch.to_string());
        self.ref_focused = true;
        self.button_focused = false;
        self.error_msg = None;
        self.dispatching = false;
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.workflow = None;
        self.fields.clear();
        self.button_focused = false;
        self.error_msg = None;
        self.dispatching = false;
    }

    /// Collect the current field values as (key, value) pairs for dispatch.
    fn collect_values(&self) -> Vec<(String, String)> {
        self.fields
            .iter()
            .map(|field| {
                let val = match &field.value {
                    FieldValue::Text(input) => input.value().to_string(),
                    FieldValue::Boolean(b) => b.to_string(),
                    FieldValue::Choice { selected, options } => {
                        options.get(*selected).cloned().unwrap_or_default()
                    }
                };
                (field.input.name.clone(), val)
            })
            .collect()
    }

    /// Validate required fields are filled.
    fn validate(&self) -> Option<String> {
        for field in &self.fields {
            if field.input.required {
                let is_empty = match &field.value {
                    FieldValue::Text(input) => input.value().trim().is_empty(),
                    FieldValue::Boolean(_) => false,
                    FieldValue::Choice { options, .. } => options.is_empty(),
                };
                if is_empty {
                    return Some(format!("'{}' is required", field.input.name));
                }
            }
        }

        if self.ref_input.value().trim().is_empty() {
            return Some("Branch/ref is required".to_string());
        }

        None
    }

    /// Total number of focusable items: ref field + each input field + dispatch button
    fn total_focusable(&self) -> usize {
        1 + self.fields.len() + 1 // ref field + input fields + button
    }

    /// Attempt to dispatch: validate, collect values, and return the action.
    fn try_dispatch(&mut self) -> Action {
        if let Some(err) = self.validate() {
            self.error_msg = Some(err);
            return Action::Noop;
        }
        self.dispatching = true;
        self.error_msg = None;
        let values = self.collect_values();
        Action::DispatchWorkflow(self.workflow_index, values)
    }
}

impl Component for DispatchForm {
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.visible || self.dispatching {
            return Action::Noop;
        }

        match key.code {
            KeyCode::Esc => {
                self.close();
                Action::CloseModal
            }
            // Tab / Shift+Tab to cycle fields
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => Action::PrevField,
            KeyCode::Tab => Action::NextField,
            // Field-specific handling
            _ => {
                if self.button_focused {
                    // Dispatch button is focused
                    match key.code {
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            return self.try_dispatch();
                        }
                        _ => {}
                    }
                } else if self.ref_focused {
                    // ref field is focused
                    match key.code {
                        KeyCode::Enter => {
                            // Move to next field
                            return Action::NextField;
                        }
                        _ => {
                            self.ref_input
                                .handle_event(&crossterm::event::Event::Key(key));
                        }
                    }
                } else {
                    // An input field is focused
                    if let Some(field) = self.fields.get_mut(self.focused_field) {
                        match &mut field.value {
                            FieldValue::Text(input) => {
                                match key.code {
                                    KeyCode::Enter => {
                                        // Move to next field
                                        return Action::NextField;
                                    }
                                    _ => {
                                        input.handle_event(&crossterm::event::Event::Key(key));
                                    }
                                }
                            }
                            FieldValue::Boolean(val) => match key.code {
                                KeyCode::Char(' ') | KeyCode::Enter => {
                                    *val = !*val;
                                }
                                _ => {}
                            },
                            FieldValue::Choice { selected, options } => match key.code {
                                KeyCode::Char('j') | KeyCode::Down => {
                                    if *selected + 1 < options.len() {
                                        *selected += 1;
                                    }
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    if *selected > 0 {
                                        *selected -= 1;
                                    }
                                }
                                KeyCode::Enter => {
                                    return Action::NextField;
                                }
                                _ => {}
                            },
                        }
                    }
                }
                self.error_msg = None;
                Action::Noop
            }
        }
    }

    fn update(&mut self, action: &Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::NextField => {
                // Order: ref -> fields[0..n] -> button -> ref (wrap)
                if self.button_focused {
                    // Wrap back to ref
                    self.button_focused = false;
                    self.ref_focused = true;
                } else if self.ref_focused {
                    self.ref_focused = false;
                    if !self.fields.is_empty() {
                        self.focused_field = 0;
                    } else {
                        self.button_focused = true;
                    }
                } else {
                    if self.focused_field + 1 < self.fields.len() {
                        self.focused_field += 1;
                    } else {
                        self.button_focused = true;
                    }
                }
            }
            Action::PrevField => {
                // Reverse: ref <- fields[0..n] <- button <- ref (wrap)
                if self.button_focused {
                    self.button_focused = false;
                    if !self.fields.is_empty() {
                        self.focused_field = self.fields.len() - 1;
                    } else {
                        self.ref_focused = true;
                    }
                } else if self.ref_focused {
                    self.ref_focused = false;
                    self.button_focused = true;
                } else {
                    if self.focused_field > 0 {
                        self.focused_field -= 1;
                    } else {
                        self.ref_focused = true;
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let workflow_name = self
            .workflow
            .as_ref()
            .map(|w| w.name.as_str())
            .unwrap_or("Workflow");

        // Calculate needed height: title(1) + border(2) + ref field(3) + fields(3 each) + button(2) + error(1) + hint(1) + padding(2)
        let field_count = self.fields.len();
        let height = (6 + field_count * 3 + 4).min(area.height as usize - 4) as u16;

        let modal_area = centered_rect(60, height, area);
        frame.render_widget(Clear, modal_area);

        let title = format!(" Dispatch: {} ", workflow_name);
        let border_color = if self.error_msg.is_some() {
            Color::Red
        } else {
            Color::Green
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        // Build constraints for all fields
        let mut constraints: Vec<Constraint> = Vec::new();
        // Ref field: label + input
        constraints.push(Constraint::Length(1)); // label
        constraints.push(Constraint::Length(1)); // input
        constraints.push(Constraint::Length(1)); // spacing

        for _ in &self.fields {
            constraints.push(Constraint::Length(1)); // label
            constraints.push(Constraint::Length(1)); // input
            constraints.push(Constraint::Length(1)); // spacing
        }

        // Button + error + hint
        constraints.push(Constraint::Length(1)); // button
        constraints.push(Constraint::Length(1)); // spacing
        constraints.push(Constraint::Length(1)); // error
        constraints.push(Constraint::Length(1)); // hint
        constraints.push(Constraint::Min(0)); // remaining space

        let chunks = Layout::vertical(constraints).split(inner);

        let mut chunk_idx = 0;

        // Ref field
        let ref_label_style = if self.ref_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let ref_label = Paragraph::new(Line::from(vec![
            Span::styled("Branch/Ref", ref_label_style),
            Span::styled(" *", Style::default().fg(Color::Red)),
        ]));
        frame.render_widget(ref_label, chunks[chunk_idx]);
        chunk_idx += 1;

        let ref_display = if self.ref_focused {
            format!("▸ {}", self.ref_input.value())
        } else {
            format!("  {}", self.ref_input.value())
        };
        let ref_paragraph =
            Paragraph::new(ref_display).style(Style::default().fg(if self.ref_focused {
                Color::Yellow
            } else {
                Color::DarkGray
            }));
        frame.render_widget(ref_paragraph, chunks[chunk_idx]);

        if self.ref_focused && !self.dispatching {
            let cursor_x = chunks[chunk_idx].x + 2 + self.ref_input.visual_cursor() as u16;
            let cursor_y = chunks[chunk_idx].y;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
        chunk_idx += 1;
        chunk_idx += 1; // spacing

        // Input fields
        for (i, field) in self.fields.iter().enumerate() {
            let is_focused = !self.ref_focused && i == self.focused_field;
            let label_style = if is_focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let mut label_spans = vec![Span::styled(&field.input.name, label_style)];
            if field.input.required {
                label_spans.push(Span::styled(" *", Style::default().fg(Color::Red)));
            }
            if !field.input.description.is_empty() {
                label_spans.push(Span::styled(
                    format!(" ({})", field.input.description),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            let label = Paragraph::new(Line::from(label_spans));
            frame.render_widget(label, chunks[chunk_idx]);
            chunk_idx += 1;

            match &field.value {
                FieldValue::Text(input) => {
                    let prefix = if is_focused { "▸ " } else { "  " };
                    let display = format!("{}{}", prefix, input.value());
                    let paragraph =
                        Paragraph::new(display).style(Style::default().fg(if is_focused {
                            Color::Yellow
                        } else {
                            Color::DarkGray
                        }));
                    frame.render_widget(paragraph, chunks[chunk_idx]);

                    if is_focused && !self.dispatching {
                        let cursor_x = chunks[chunk_idx].x + 2 + input.visual_cursor() as u16;
                        let cursor_y = chunks[chunk_idx].y;
                        frame.set_cursor_position((cursor_x, cursor_y));
                    }
                }
                FieldValue::Boolean(val) => {
                    let checkbox = if *val { "[x]" } else { "[ ]" };
                    let prefix = if is_focused { "▸ " } else { "  " };
                    let display = format!(
                        "{}{} {}",
                        prefix,
                        checkbox,
                        if *val { "true" } else { "false" }
                    );
                    let paragraph =
                        Paragraph::new(display).style(Style::default().fg(if is_focused {
                            Color::Yellow
                        } else {
                            Color::DarkGray
                        }));
                    frame.render_widget(paragraph, chunks[chunk_idx]);
                }
                FieldValue::Choice { selected, options } => {
                    let display = if let Some(opt) = options.get(*selected) {
                        let prefix = if is_focused { "▸ " } else { "  " };
                        format!("{}{} ({}/{})", prefix, opt, selected + 1, options.len())
                    } else {
                        "  (no options)".to_string()
                    };
                    let paragraph =
                        Paragraph::new(display).style(Style::default().fg(if is_focused {
                            Color::Yellow
                        } else {
                            Color::DarkGray
                        }));
                    frame.render_widget(paragraph, chunks[chunk_idx]);
                }
            }
            chunk_idx += 1;
            chunk_idx += 1; // spacing
        }

        // Dispatch button
        if chunk_idx < chunks.len() {
            let button_style = if self.button_focused {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            let button_text = if self.button_focused {
                "  [ Dispatch ]  "
            } else {
                "  [ Dispatch ]  "
            };
            let button = Paragraph::new(button_text).style(button_style);
            frame.render_widget(button, chunks[chunk_idx]);
        }
        chunk_idx += 1;
        chunk_idx += 1; // spacing

        // Error message
        if let Some(ref err) = self.error_msg {
            let err_paragraph = Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red));
            if chunk_idx < chunks.len() {
                frame.render_widget(err_paragraph, chunks[chunk_idx]);
            }
        }
        chunk_idx += 1;

        // Hint
        if chunk_idx < chunks.len() {
            let hint = if self.dispatching {
                Paragraph::new("Dispatching...").style(Style::default().fg(Color::Yellow))
            } else {
                Paragraph::new("Tab: next field | Enter: dispatch | Esc: cancel")
                    .style(Style::default().fg(Color::DarkGray))
            };
            frame.render_widget(hint, chunks[chunk_idx]);
        }
    }
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}
