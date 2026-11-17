#![allow(dead_code)]

use std::path::PathBuf;

use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::KeyCode;

use crate::api::ApiClient;
use crate::checklist::{
    append_to_note, parse_checklist_items, toggle_checklist_item_by_index, ChecklistItem,
};
use crate::config::{get_preset_hosts, Config};
use crate::models::{Note, NoteDetail, NoteInput, Project, ProjectInput, Stats};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Sidebar,
    Notes,
    Detail,
}

impl Pane {
    pub fn next(self) -> Self {
        match self {
            Pane::Sidebar => Pane::Notes,
            Pane::Notes => Pane::Detail,
            Pane::Detail => Pane::Sidebar,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Pane::Sidebar => Pane::Detail,
            Pane::Notes => Pane::Sidebar,
            Pane::Detail => Pane::Notes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Preview,
    Raw,
}

impl ViewMode {
    pub fn toggle(self) -> Self {
        match self {
            ViewMode::Preview => ViewMode::Raw,
            ViewMode::Raw => ViewMode::Preview,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Preview => "Preview",
            ViewMode::Raw => "Raw",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeItem {
    All,
    Pinned,
    Standalone,
    Project(Project),
    PinnedNote(Note),
}

#[derive(Debug, Clone)]
pub enum DeleteTarget {
    Project(Project),
    Note(Note),
    Attachment {
        note_id: u64,
        attachment_id: u64,
        filename: String,
    },
}

#[derive(Debug, Clone)]
pub enum Modal {
    HostSwitcher {
        cursor: usize,
        custom_input: String,
        entering_custom: bool,
    },
    Search {
        input: String,
    },
    QuickAppendNote {
        content_input: String,
        section_input: String,
        as_task: bool,
        focus_field: usize, // 0: content, 1: section, 2: as_task
    },
    ProjectForm {
        id: Option<u64>,
        name_input: String,
        desc_input: String,
        focus_desc: bool,
    },
    NoteForm {
        id: Option<u64>,
        title_input: String,
        project_choice_idx: usize, // 0 = None (Standalone), 1.. = index into projects
        pinned: bool,
        focus_field: usize, // 0: title, 1: project, 2: pinned
    },
    ConfirmDelete {
        target: DeleteTarget,
    },
    Attachments {
        cursor: usize,
        upload_input: Option<String>,
        dest_input: Option<String>,
    },
    Help,
}

pub struct App {
    pub config: Config,
    pub client: ApiClient,
    pub should_quit: bool,
    pub pending_editor: Option<(u64, PathBuf)>, // note_id, temp_path
    pub status: Option<(String, bool)>,         // (message, is_error)

    // Layout & Navigation
    pub focused_pane: Pane,
    pub view_mode: ViewMode,
    pub modal: Option<Modal>,

    // Data
    pub stats: Stats,
    pub preset_hosts: Vec<String>,
    pub projects: Vec<Project>,
    pub pinned_notes: Vec<Note>,
    pub notes: Vec<Note>,
    pub current_note: Option<NoteDetail>,
    pub checklist_items: Vec<ChecklistItem>,

    // Selections
    pub sidebar_selected: usize,
    pub notes_selected: usize,
    pub detail_selected_checklist: Option<usize>,
    pub detail_scroll: usize,

    // Search query
    pub search_query: Option<String>,

    pub clipboard: Option<Clipboard>,
}

pub fn sort_notes(notes: &mut [Note]) {
    notes.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.id.cmp(&b.id))
    });
}

impl App {
    pub fn new(config: Config) -> Self {
        let client = ApiClient::new(&config);
        let clipboard = Clipboard::new().ok();

        let mut app = Self {
            config,
            client,
            should_quit: false,
            pending_editor: None,
            status: None,

            focused_pane: Pane::Notes,
            view_mode: ViewMode::Preview,
            modal: None,

            stats: Stats::default(),
            preset_hosts: get_preset_hosts(),
            projects: Vec::new(),
            pinned_notes: Vec::new(),
            notes: Vec::new(),
            current_note: None,
            checklist_items: Vec::new(),

            sidebar_selected: 0,
            notes_selected: 0,
            detail_selected_checklist: None,
            detail_scroll: 0,

            search_query: None,
            clipboard,
        };

        app.reload_all();
        app
    }

    pub fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status = Some((msg.into(), is_error));
    }

    pub fn clear_status(&mut self) {
        self.status = None;
    }

    // Host Management
    pub fn switch_host(&mut self, new_url: &str) {
        let normalized = crate::config::normalize_url(new_url);
        self.config.set_url(&normalized);
        self.client.set_base_url(&normalized);
        self.set_status(format!("Switched host to {}", normalized), false);
        self.reload_all();
    }

    // Data Fetching
    pub fn reload_all(&mut self) {
        self.load_stats();
        self.load_projects();
        self.load_pinned_notes();
        self.load_notes();
    }

    pub fn load_stats(&mut self) {
        match self.client.get_stats() {
            Ok(s) => self.stats = s,
            Err(e) => self.set_status(format!("Stats error: {}", e), true),
        }
    }

    pub fn load_projects(&mut self) {
        match self.client.list_projects(None) {
            Ok(projs) => {
                self.projects = projs;
                if self.sidebar_selected >= self.total_sidebar_items() {
                    self.sidebar_selected = self.total_sidebar_items().saturating_sub(1);
                }
            }
            Err(e) => self.set_status(format!("Projects error: {}", e), true),
        }
    }

    pub fn load_pinned_notes(&mut self) {
        match self.client.list_notes(None, None, Some(true), None) {
            Ok(mut notes) => {
                sort_notes(&mut notes);
                self.pinned_notes = notes;
                if self.sidebar_selected >= self.total_sidebar_items() {
                    self.sidebar_selected = self.total_sidebar_items().saturating_sub(1);
                }
            }
            Err(e) => self.set_status(format!("Pinned notes error: {}", e), true),
        }
    }

    pub fn total_sidebar_items(&self) -> usize {
        3 + self.projects.len() + self.pinned_notes.len()
    }

    pub fn get_sidebar_item(&self, idx: usize) -> ScopeItem {
        match idx {
            0 => ScopeItem::All,
            1 => ScopeItem::Pinned,
            2 => ScopeItem::Standalone,
            n => {
                let proj_idx = n - 3;
                if proj_idx < self.projects.len() {
                    ScopeItem::Project(self.projects[proj_idx].clone())
                } else {
                    let pinned_idx = proj_idx - self.projects.len();
                    if let Some(note) = self.pinned_notes.get(pinned_idx) {
                        ScopeItem::PinnedNote(note.clone())
                    } else {
                        ScopeItem::All
                    }
                }
            }
        }
    }

    pub fn load_notes(&mut self) {
        let scope_item = self.get_sidebar_item(self.sidebar_selected);
        let q = self.search_query.as_deref();

        let res = match scope_item {
            ScopeItem::All => self.client.list_notes(None, None, None, q),
            ScopeItem::Pinned => self.client.list_notes(None, None, Some(true), q),
            ScopeItem::Standalone => self.client.list_notes(None, Some("standalone"), None, q),
            ScopeItem::Project(p) => self.client.list_notes(Some(p.id), None, None, q),
            ScopeItem::PinnedNote(note) => {
                if let Some(query) = q {
                    let query_lower = query.to_lowercase();
                    if note.title.to_lowercase().contains(&query_lower) {
                        Ok(vec![note])
                    } else {
                        Ok(vec![])
                    }
                } else {
                    Ok(vec![note])
                }
            }
        };

        match res {
            Ok(mut list) => {
                sort_notes(&mut list);
                self.notes = list;
                if self.notes.is_empty() {
                    self.notes_selected = 0;
                    self.current_note = None;
                    self.checklist_items.clear();
                    self.detail_selected_checklist = None;
                    self.detail_scroll = 0;
                } else {
                    if self.notes_selected >= self.notes.len() {
                        self.notes_selected = self.notes.len() - 1;
                    }
                    self.load_current_note_detail();
                }
            }
            Err(e) => self.set_status(format!("Notes error: {}", e), true),
        }
    }

    pub fn load_current_note_detail(&mut self) {
        if let Some(note) = self.notes.get(self.notes_selected) {
            match self.client.get_note(note.id) {
                Ok(detail) => {
                    self.checklist_items =
                        parse_checklist_items(detail.body.as_deref().unwrap_or(""));
                    if self.checklist_items.is_empty() {
                        self.detail_selected_checklist = None;
                    } else if let Some(cur) = self.detail_selected_checklist {
                        if cur >= self.checklist_items.len() {
                            self.detail_selected_checklist = Some(self.checklist_items.len() - 1);
                        }
                    } else {
                        self.detail_selected_checklist = Some(0);
                    }
                    self.detail_scroll = 0;
                    self.current_note = Some(detail);
                }
                Err(e) => {
                    self.set_status(format!("Detail load error: {}", e), true);
                    self.current_note = None;
                    self.checklist_items.clear();
                }
            }
        } else {
            self.current_note = None;
            self.checklist_items.clear();
            self.detail_selected_checklist = None;
            self.detail_scroll = 0;
        }
    }

    // Toggle Checklist Item
    pub fn toggle_current_checklist_item(&mut self) {
        let (detail, cur_idx) = match (&self.current_note, self.detail_selected_checklist) {
            (Some(d), Some(idx)) => (d.clone(), idx),
            _ => return,
        };

        let body = detail.body.as_deref().unwrap_or("");
        if let Some((updated_body, item)) = toggle_checklist_item_by_index(body, cur_idx) {
            let input = NoteInput {
                title: detail.title.clone(),
                body: Some(updated_body.clone()),
                ..Default::default()
            };

            match self.client.update_note(detail.id, &input) {
                Ok(new_detail) => {
                    if let Some(list_item) = self.notes.get_mut(self.notes_selected) {
                        list_item.body = new_detail.body.clone();
                    }
                    self.checklist_items = parse_checklist_items(&updated_body);
                    self.current_note = Some(new_detail);
                    let state_str = if item.completed { "done" } else { "pending" };
                    self.set_status(format!("Task marked {}", state_str), false);
                }
                Err(e) => self.set_status(format!("Failed to update note: {}", e), true),
            }
        }
    }

    // Quick Append to Note
    pub fn append_note_content(&mut self, text: &str, section: Option<&str>, as_task: bool) {
        let detail = match &self.current_note {
            Some(d) => d.clone(),
            None => return,
        };

        let current_body = detail.body.as_deref().unwrap_or("");
        let updated = append_to_note(current_body, text, section, as_task);

        let input = NoteInput {
            title: detail.title.clone(),
            body: Some(updated.clone()),
            ..Default::default()
        };

        match self.client.update_note(detail.id, &input) {
            Ok(new_detail) => {
                if let Some(list_item) = self.notes.get_mut(self.notes_selected) {
                    list_item.body = new_detail.body.clone();
                }
                self.checklist_items = parse_checklist_items(&updated);
                if !self.checklist_items.is_empty() && self.detail_selected_checklist.is_none() {
                    self.detail_selected_checklist = Some(0);
                }
                self.current_note = Some(new_detail);
                self.set_status("Appended to note successfully", false);
            }
            Err(e) => self.set_status(format!("Failed to append to note: {}", e), true),
        }
    }

    // Toggle Pin on selected Note
    pub fn toggle_pin_current(&mut self) {
        if let Some(note) = self.notes.get(self.notes_selected) {
            match self.client.toggle_pin(note.id) {
                Ok(resp) => {
                    let msg = if resp.pinned {
                        "Pinned note"
                    } else {
                        "Unpinned note"
                    };
                    self.set_status(msg, false);
                    self.load_stats();
                    self.load_pinned_notes();
                    self.load_notes();
                }
                Err(e) => self.set_status(format!("Toggle pin error: {}", e), true),
            }
        }
    }

    // Reorder pinned notes up/down
    pub fn move_pinned_current(&mut self, direction_up: bool) {
        let is_pinned_scope =
            matches!(self.get_sidebar_item(self.sidebar_selected), ScopeItem::Pinned);
        if !is_pinned_scope || self.notes.len() < 2 {
            return;
        }

        let cur_idx = self.notes_selected;
        let new_idx = if direction_up {
            if cur_idx == 0 {
                return;
            }
            cur_idx - 1
        } else {
            if cur_idx + 1 >= self.notes.len() {
                return;
            }
            cur_idx + 1
        };

        self.notes.swap(cur_idx, new_idx);
        self.notes_selected = new_idx;

        let ids: Vec<u64> = self.notes.iter().map(|n| n.id).collect();
        if let Err(e) = self.client.reorder_pinned(&ids) {
            self.set_status(format!("Reorder error: {}", e), true);
        } else {
            self.set_status("Reordered pinned notes", false);
        }
    }

    // Yank to clipboard
    pub fn yank_current(&mut self, full_body: bool) {
        let text_to_copy = if full_body {
            self.current_note
                .as_ref()
                .and_then(|d| d.body.clone())
                .unwrap_or_default()
        } else if let Some(idx) = self.detail_selected_checklist {
            self.checklist_items
                .get(idx)
                .map(|item| item.text.clone())
                .unwrap_or_default()
        } else {
            self.current_note
                .as_ref()
                .map(|d| d.title.clone())
                .unwrap_or_default()
        };

        if text_to_copy.is_empty() {
            self.set_status("Nothing to copy", false);
            return;
        }

        if let Some(cb) = &mut self.clipboard {
            if let Err(e) = cb.set_text(text_to_copy.clone()) {
                self.set_status(format!("Clipboard error: {}", e), true);
            } else {
                let first_line = text_to_copy.lines().next().unwrap_or("");
                let preview = if first_line.chars().count() > 30 {
                    let s: String = first_line.chars().take(30).collect();
                    format!("{}...", s)
                } else {
                    first_line.to_string()
                };
                if full_body {
                    self.set_status(format!("Copied Markdown: \"{}\"", preview), false);
                } else {
                    self.set_status(format!("Copied: \"{}\"", preview), false);
                }
            }
        } else {
            self.set_status("Clipboard unavailable", true);
        }
    }

    // External Editor Setup
    pub fn prepare_external_editor(&mut self) -> Result<()> {
        let detail = match &self.current_note {
            Some(d) => d.clone(),
            None => return Ok(()),
        };

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("noteking-note-{}.md", detail.id));
        std::fs::write(&file_path, detail.body.as_deref().unwrap_or(""))?;

        self.pending_editor = Some((detail.id, file_path));
        Ok(())
    }

    pub fn complete_external_editor(&mut self) -> Result<()> {
        if let Some((id, path)) = self.pending_editor.take() {
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let _ = std::fs::remove_file(&path);

                if let Some(detail) = &self.current_note {
                    if detail.id == id {
                        let input = NoteInput {
                            title: detail.title.clone(),
                            body: Some(content.clone()),
                            ..Default::default()
                        };
                        match self.client.update_note(id, &input) {
                            Ok(new_detail) => {
                                if let Some(list_item) = self.notes.get_mut(self.notes_selected) {
                                    list_item.body = new_detail.body.clone();
                                }
                                self.checklist_items = parse_checklist_items(&content);
                                self.current_note = Some(new_detail);
                                self.set_status("Saved note markdown from editor", false);
                            }
                            Err(e) => {
                                self.set_status(format!("Failed to save from editor: {}", e), true)
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // Keyboard Event Handling
    pub fn handle_key(&mut self, key: KeyCode) {
        if self.modal.is_some() {
            self.handle_modal_key(key);
            return;
        }

        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.modal = Some(Modal::Help),
            KeyCode::Char('H') | KeyCode::Char('u') => {
                if self.config.allow_host_switcher {
                    self.modal = Some(Modal::HostSwitcher {
                        cursor: 0,
                        custom_input: self.config.base_url.clone(),
                        entering_custom: false,
                    });
                } else {
                    self.set_status(
                        "Host switching locked by NOTEKING_REMOTE_URL (pass --allow-switch to enable)",
                        true,
                    );
                }
            }
            KeyCode::Char('/') => {
                self.modal = Some(Modal::Search {
                    input: self.search_query.clone().unwrap_or_default(),
                });
            }
            KeyCode::Char('r') => {
                self.reload_all();
                self.set_status("Refreshed all data", false);
            }
            KeyCode::Char('v') | KeyCode::Char('m') => {
                self.view_mode = self.view_mode.toggle();
                self.set_status(
                    format!("Switched to {} Markdown mode", self.view_mode.label()),
                    false,
                );
            }

            // Pane Navigation
            KeyCode::Char('1') => self.focused_pane = Pane::Sidebar,
            KeyCode::Char('2') => self.focused_pane = Pane::Notes,
            KeyCode::Char('3') => self.focused_pane = Pane::Detail,
            KeyCode::Tab => self.focused_pane = self.focused_pane.next(),
            KeyCode::BackTab => self.focused_pane = self.focused_pane.prev(),
            KeyCode::Char('h') | KeyCode::Left => self.focused_pane = self.focused_pane.prev(),
            KeyCode::Char('l') | KeyCode::Right => self.focused_pane = self.focused_pane.next(),

            // Actions per pane
            _ => match self.focused_pane {
                Pane::Sidebar => self.handle_sidebar_key(key),
                Pane::Notes => self.handle_notes_key(key),
                Pane::Detail => self.handle_detail_key(key),
            },
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.sidebar_selected + 1 < self.total_sidebar_items() {
                    self.sidebar_selected += 1;
                    self.notes_selected = 0;
                    self.load_notes();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.sidebar_selected > 0 {
                    self.sidebar_selected -= 1;
                    self.notes_selected = 0;
                    self.load_notes();
                }
            }
            KeyCode::Enter => match self.get_sidebar_item(self.sidebar_selected) {
                ScopeItem::PinnedNote(_) => {
                    self.focused_pane = Pane::Detail;
                }
                _ => {
                    self.focused_pane = Pane::Notes;
                }
            },
            KeyCode::Char('n') => {
                self.modal = Some(Modal::ProjectForm {
                    id: None,
                    name_input: String::new(),
                    desc_input: String::new(),
                    focus_desc: false,
                });
            }
            KeyCode::Char('e') => {
                if let ScopeItem::Project(p) = self.get_sidebar_item(self.sidebar_selected) {
                    self.modal = Some(Modal::ProjectForm {
                        id: Some(p.id),
                        name_input: p.name,
                        desc_input: p.description.unwrap_or_default(),
                        focus_desc: false,
                    });
                }
            }
            KeyCode::Char('d') => {
                if let ScopeItem::Project(p) = self.get_sidebar_item(self.sidebar_selected) {
                    self.modal = Some(Modal::ConfirmDelete {
                        target: DeleteTarget::Project(p),
                    });
                }
            }
            _ => {}
        }
    }

    fn handle_notes_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.notes.is_empty() && self.notes_selected + 1 < self.notes.len() {
                    self.notes_selected += 1;
                    self.load_current_note_detail();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.notes_selected > 0 {
                    self.notes_selected -= 1;
                    self.load_current_note_detail();
                }
            }
            KeyCode::Enter => {
                self.focused_pane = Pane::Detail;
            }
            KeyCode::Char('p') => {
                self.toggle_pin_current();
            }
            KeyCode::Char('K') => {
                self.move_pinned_current(true);
            }
            KeyCode::Char('J') => {
                self.move_pinned_current(false);
            }
            KeyCode::Char('y') => {
                self.yank_current(false);
            }
            KeyCode::Char('Y') | KeyCode::Char('c') => {
                self.yank_current(true);
            }
            KeyCode::Char('E') => {
                if let Err(e) = self.prepare_external_editor() {
                    self.set_status(format!("Failed to open editor: {}", e), true);
                }
            }
            KeyCode::Char('a') => {
                if self.current_note.is_some() {
                    self.modal = Some(Modal::QuickAppendNote {
                        content_input: String::new(),
                        section_input: String::new(),
                        as_task: false,
                        focus_field: 0,
                    });
                }
            }
            KeyCode::Char('A') => {
                if self.current_note.is_some() {
                    self.modal = Some(Modal::Attachments {
                        cursor: 0,
                        upload_input: None,
                        dest_input: None,
                    });
                }
            }
            KeyCode::Char('n') => {
                let default_proj_idx = match self.get_sidebar_item(self.sidebar_selected) {
                    ScopeItem::Project(p) => self
                        .projects
                        .iter()
                        .position(|pr| pr.id == p.id)
                        .map(|i| i + 1)
                        .unwrap_or(0),
                    _ => 0,
                };
                self.modal = Some(Modal::NoteForm {
                    id: None,
                    title_input: String::new(),
                    project_choice_idx: default_proj_idx,
                    pinned: false,
                    focus_field: 0,
                });
            }
            KeyCode::Char('e') => {
                if let Some(detail) = &self.current_note {
                    let proj_choice = detail
                        .project_id
                        .and_then(|pid| {
                            self.projects
                                .iter()
                                .position(|p| p.id == pid)
                                .map(|i| i + 1)
                        })
                        .unwrap_or(0);
                    self.modal = Some(Modal::NoteForm {
                        id: Some(detail.id),
                        title_input: detail.title.clone(),
                        project_choice_idx: proj_choice,
                        pinned: detail.pinned,
                        focus_field: 0,
                    });
                }
            }
            KeyCode::Char('d') => {
                if let Some(note) = self.notes.get(self.notes_selected) {
                    self.modal = Some(Modal::ConfirmDelete {
                        target: DeleteTarget::Note(note.clone()),
                    });
                }
            }
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.checklist_items.is_empty() && self.view_mode == ViewMode::Preview {
                    let next = match self.detail_selected_checklist {
                        Some(i) if i + 1 < self.checklist_items.len() => i + 1,
                        None => 0,
                        Some(i) => {
                            self.detail_scroll = self.detail_scroll.saturating_add(1);
                            i
                        }
                    };
                    self.detail_selected_checklist = Some(next);
                    self.adjust_scroll_to_checklist_item(next);
                } else {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.checklist_items.is_empty() && self.view_mode == ViewMode::Preview {
                    let prev = match self.detail_selected_checklist {
                        Some(i) if i > 0 => i - 1,
                        _ => {
                            self.detail_scroll = self.detail_scroll.saturating_sub(1);
                            0
                        }
                    };
                    self.detail_selected_checklist = Some(prev);
                    self.adjust_scroll_to_checklist_item(prev);
                } else {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                }
            }
            // Document scrolling
            KeyCode::Char('J') => {
                self.detail_scroll = self.detail_scroll.saturating_add(3);
            }
            KeyCode::Char('K') => {
                self.detail_scroll = self.detail_scroll.saturating_sub(3);
            }
            KeyCode::PageDown => {
                self.detail_scroll = self.detail_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.detail_scroll = self.detail_scroll.saturating_sub(10);
            }
            KeyCode::Char('g') => {
                self.detail_scroll = 0;
                if !self.checklist_items.is_empty() {
                    self.detail_selected_checklist = Some(0);
                }
            }
            KeyCode::Char('G') => {
                let total_lines = self
                    .current_note
                    .as_ref()
                    .and_then(|d| d.body.as_deref())
                    .map(|b| b.lines().count())
                    .unwrap_or(0);
                self.detail_scroll = total_lines.saturating_sub(5);
                if !self.checklist_items.is_empty() {
                    self.detail_selected_checklist = Some(self.checklist_items.len() - 1);
                }
            }
            // Checklist toggle
            KeyCode::Char(' ') | KeyCode::Enter => {
                if !self.checklist_items.is_empty() {
                    self.toggle_current_checklist_item();
                }
            }
            // Quick append to note
            KeyCode::Char('a') => {
                if self.current_note.is_some() {
                    self.modal = Some(Modal::QuickAppendNote {
                        content_input: String::new(),
                        section_input: String::new(),
                        as_task: false,
                        focus_field: 0,
                    });
                }
            }
            // External editor
            KeyCode::Char('E') => {
                if let Err(e) = self.prepare_external_editor() {
                    self.set_status(format!("Failed to open editor: {}", e), true);
                }
            }
            // Yank
            KeyCode::Char('y') => {
                self.yank_current(false);
            }
            KeyCode::Char('Y') | KeyCode::Char('c') => {
                self.yank_current(true);
            }
            // Pin toggle
            KeyCode::Char('p') => {
                self.toggle_pin_current();
            }
            // Attachments
            KeyCode::Char('A') => {
                if self.current_note.is_some() {
                    self.modal = Some(Modal::Attachments {
                        cursor: 0,
                        upload_input: None,
                        dest_input: None,
                    });
                }
            }
            // Edit Note metadata
            KeyCode::Char('e') => {
                if let Some(detail) = &self.current_note {
                    let proj_choice = detail
                        .project_id
                        .and_then(|pid| {
                            self.projects
                                .iter()
                                .position(|p| p.id == pid)
                                .map(|i| i + 1)
                        })
                        .unwrap_or(0);
                    self.modal = Some(Modal::NoteForm {
                        id: Some(detail.id),
                        title_input: detail.title.clone(),
                        project_choice_idx: proj_choice,
                        pinned: detail.pinned,
                        focus_field: 0,
                    });
                }
            }
            _ => {}
        }
    }

    fn adjust_scroll_to_checklist_item(&mut self, item_idx: usize) {
        if let Some(item) = self.checklist_items.get(item_idx) {
            let line = item.line_number.saturating_sub(1);
            if line < self.detail_scroll {
                self.detail_scroll = line;
            } else if line > self.detail_scroll + 15 {
                self.detail_scroll = line.saturating_sub(10);
            }
        }
    }

    // Modal Event Handling
    fn handle_modal_key(&mut self, key: KeyCode) {
        let modal = self.modal.clone();
        match modal {
            Some(Modal::HostSwitcher {
                cursor,
                mut custom_input,
                entering_custom,
            }) => match key {
                KeyCode::Esc => self.modal = None,
                KeyCode::Up => {
                    if !entering_custom && cursor > 0 {
                        self.modal = Some(Modal::HostSwitcher {
                            cursor: cursor - 1,
                            custom_input,
                            entering_custom: false,
                        });
                    }
                }
                KeyCode::Down => {
                    if !entering_custom && cursor < self.preset_hosts.len() {
                        self.modal = Some(Modal::HostSwitcher {
                            cursor: cursor + 1,
                            custom_input,
                            entering_custom: false,
                        });
                    }
                }
                KeyCode::Tab => {
                    let next_entering = !entering_custom;
                    self.modal = Some(Modal::HostSwitcher {
                        cursor,
                        custom_input,
                        entering_custom: next_entering,
                    });
                }
                KeyCode::Char(c) if entering_custom => {
                    custom_input.push(c);
                    self.modal = Some(Modal::HostSwitcher {
                        cursor,
                        custom_input,
                        entering_custom: true,
                    });
                }
                KeyCode::Backspace if entering_custom => {
                    custom_input.pop();
                    self.modal = Some(Modal::HostSwitcher {
                        cursor,
                        custom_input,
                        entering_custom: true,
                    });
                }
                KeyCode::Enter => {
                    self.modal = None;
                    if entering_custom || cursor == self.preset_hosts.len() {
                        if !custom_input.trim().is_empty() {
                            self.switch_host(&custom_input);
                        }
                    } else if let Some(preset) = self.preset_hosts.get(cursor).cloned() {
                        self.switch_host(&preset);
                    }
                }
                _ => {}
            },

            Some(Modal::Search { mut input }) => match key {
                KeyCode::Esc => {
                    self.modal = None;
                    self.search_query = None;
                    self.load_notes();
                }
                KeyCode::Enter => {
                    self.modal = None;
                    self.search_query = if input.trim().is_empty() {
                        None
                    } else {
                        Some(input.trim().to_string())
                    };
                    self.load_notes();
                }
                KeyCode::Char(c) => {
                    input.push(c);
                    self.modal = Some(Modal::Search {
                        input: input.clone(),
                    });
                    self.search_query = Some(input);
                    self.load_notes();
                }
                KeyCode::Backspace => {
                    input.pop();
                    self.modal = Some(Modal::Search {
                        input: input.clone(),
                    });
                    self.search_query = if input.is_empty() { None } else { Some(input) };
                    self.load_notes();
                }
                _ => {}
            },

            Some(Modal::QuickAppendNote {
                mut content_input,
                mut section_input,
                mut as_task,
                focus_field,
            }) => match key {
                KeyCode::Esc => self.modal = None,
                KeyCode::Tab => {
                    self.modal = Some(Modal::QuickAppendNote {
                        content_input,
                        section_input,
                        as_task,
                        focus_field: (focus_field + 1) % 3,
                    });
                }
                KeyCode::BackTab => {
                    self.modal = Some(Modal::QuickAppendNote {
                        content_input,
                        section_input,
                        as_task,
                        focus_field: (focus_field + 2) % 3,
                    });
                }
                KeyCode::Char(' ') if focus_field == 2 => {
                    as_task = !as_task;
                    self.modal = Some(Modal::QuickAppendNote {
                        content_input,
                        section_input,
                        as_task,
                        focus_field,
                    });
                }
                KeyCode::Enter => {
                    if !content_input.trim().is_empty() {
                        let sec = if section_input.trim().is_empty() {
                            None
                        } else {
                            Some(section_input.trim())
                        };
                        self.append_note_content(content_input.trim(), sec, as_task);
                    }
                    self.modal = None;
                }
                KeyCode::Char(c) => {
                    if focus_field == 0 {
                        content_input.push(c);
                    } else if focus_field == 1 {
                        section_input.push(c);
                    }
                    self.modal = Some(Modal::QuickAppendNote {
                        content_input,
                        section_input,
                        as_task,
                        focus_field,
                    });
                }
                KeyCode::Backspace => {
                    if focus_field == 0 {
                        content_input.pop();
                    } else if focus_field == 1 {
                        section_input.pop();
                    }
                    self.modal = Some(Modal::QuickAppendNote {
                        content_input,
                        section_input,
                        as_task,
                        focus_field,
                    });
                }
                _ => {}
            },

            Some(Modal::ProjectForm {
                id,
                mut name_input,
                mut desc_input,
                focus_desc,
            }) => match key {
                KeyCode::Esc => self.modal = None,
                KeyCode::Tab | KeyCode::BackTab => {
                    self.modal = Some(Modal::ProjectForm {
                        id,
                        name_input,
                        desc_input,
                        focus_desc: !focus_desc,
                    });
                }
                KeyCode::Enter => {
                    if !name_input.trim().is_empty() {
                        let input = ProjectInput {
                            name: name_input.trim().to_string(),
                            description: if desc_input.trim().is_empty() {
                                None
                            } else {
                                Some(desc_input.trim().to_string())
                            },
                        };

                        let res = match id {
                            Some(pid) => self.client.update_project(pid, &input),
                            None => self.client.create_project(&input),
                        };

                        match res {
                            Ok(p) => {
                                self.set_status(format!("Saved project '{}'", p.name), false);
                                self.load_stats();
                                self.load_projects();
                            }
                            Err(e) => self.set_status(format!("Project save error: {}", e), true),
                        }
                    }
                    self.modal = None;
                }
                KeyCode::Char(c) => {
                    if focus_desc {
                        desc_input.push(c);
                    } else {
                        name_input.push(c);
                    }
                    self.modal = Some(Modal::ProjectForm {
                        id,
                        name_input,
                        desc_input,
                        focus_desc,
                    });
                }
                KeyCode::Backspace => {
                    if focus_desc {
                        desc_input.pop();
                    } else {
                        name_input.pop();
                    }
                    self.modal = Some(Modal::ProjectForm {
                        id,
                        name_input,
                        desc_input,
                        focus_desc,
                    });
                }
                _ => {}
            },

            Some(Modal::NoteForm {
                id,
                mut title_input,
                project_choice_idx,
                mut pinned,
                focus_field,
            }) => match key {
                KeyCode::Esc => self.modal = None,
                KeyCode::Tab => {
                    let next = (focus_field + 1) % 3;
                    self.modal = Some(Modal::NoteForm {
                        id,
                        title_input,
                        project_choice_idx,
                        pinned,
                        focus_field: next,
                    });
                }
                KeyCode::BackTab => {
                    let prev = (focus_field + 2) % 3;
                    self.modal = Some(Modal::NoteForm {
                        id,
                        title_input,
                        project_choice_idx,
                        pinned,
                        focus_field: prev,
                    });
                }
                KeyCode::Left if focus_field == 1 => {
                    let next_idx = if project_choice_idx > 0 {
                        project_choice_idx - 1
                    } else {
                        self.projects.len()
                    };
                    self.modal = Some(Modal::NoteForm {
                        id,
                        title_input,
                        project_choice_idx: next_idx,
                        pinned,
                        focus_field,
                    });
                }
                KeyCode::Right if focus_field == 1 => {
                    let next_idx = (project_choice_idx + 1) % (self.projects.len() + 1);
                    self.modal = Some(Modal::NoteForm {
                        id,
                        title_input,
                        project_choice_idx: next_idx,
                        pinned,
                        focus_field,
                    });
                }
                KeyCode::Char(' ') if focus_field == 2 => {
                    pinned = !pinned;
                    self.modal = Some(Modal::NoteForm {
                        id,
                        title_input,
                        project_choice_idx,
                        pinned,
                        focus_field,
                    });
                }
                KeyCode::Enter => {
                    if !title_input.trim().is_empty() {
                        let selected_pid = if project_choice_idx == 0 {
                            None
                        } else {
                            self.projects.get(project_choice_idx - 1).map(|p| p.id)
                        };

                        let input = NoteInput {
                            title: title_input.trim().to_string(),
                            project_id: Some(selected_pid),
                            pinned: Some(pinned),
                            ..Default::default()
                        };

                        let res = match id {
                            Some(nid) => self.client.update_note(nid, &input),
                            None => self.client.create_note(&input),
                        };

                        match res {
                            Ok(n) => {
                                self.set_status(format!("Saved note '{}'", n.title), false);
                                self.load_stats();
                                self.load_projects();
                                self.load_pinned_notes();
                                self.load_notes();
                            }
                            Err(e) => self.set_status(format!("Note save error: {}", e), true),
                        }
                    }
                    self.modal = None;
                }
                KeyCode::Char(c) if focus_field == 0 => {
                    title_input.push(c);
                    self.modal = Some(Modal::NoteForm {
                        id,
                        title_input,
                        project_choice_idx,
                        pinned,
                        focus_field,
                    });
                }
                KeyCode::Backspace if focus_field == 0 => {
                    title_input.pop();
                    self.modal = Some(Modal::NoteForm {
                        id,
                        title_input,
                        project_choice_idx,
                        pinned,
                        focus_field,
                    });
                }
                _ => {}
            },

            Some(Modal::ConfirmDelete { target }) => match key {
                KeyCode::Esc | KeyCode::Char('n') => self.modal = None,
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.modal = None;
                    match target {
                        DeleteTarget::Project(p) => match self.client.delete_project(p.id) {
                            Ok(_) => {
                                self.set_status(format!("Deleted project '{}'", p.name), false);
                                self.sidebar_selected = 0;
                                self.load_stats();
                                self.load_projects();
                                self.load_pinned_notes();
                                self.load_notes();
                            }
                            Err(e) => {
                                self.set_status(format!("Delete project error: {}", e), true)
                            }
                        },
                        DeleteTarget::Note(n) => match self.client.delete_note(n.id) {
                            Ok(_) => {
                                self.set_status(format!("Deleted note '{}'", n.title), false);
                                self.load_stats();
                                self.load_projects();
                                self.load_pinned_notes();
                                self.load_notes();
                            }
                            Err(e) => {
                                self.set_status(format!("Delete note error: {}", e), true)
                            }
                        },
                        DeleteTarget::Attachment {
                            note_id,
                            attachment_id,
                            filename,
                        } => match self.client.delete_attachment(note_id, attachment_id) {
                            Ok(_) => {
                                self.set_status(format!("Deleted attachment {}", filename), false);
                                self.load_stats();
                                self.load_current_note_detail();
                            }
                            Err(e) => {
                                self.set_status(format!("Delete attachment error: {}", e), true)
                            }
                        },
                    }
                }
                _ => {}
            },

            Some(Modal::Attachments {
                cursor,
                upload_input,
                dest_input,
            }) => {
                let attachments = self
                    .current_note
                    .as_ref()
                    .map(|d| d.attachments.clone())
                    .unwrap_or_default();

                if let Some(mut path_str) = upload_input {
                    match key {
                        KeyCode::Esc => {
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                        KeyCode::Enter => {
                            let p = PathBuf::from(path_str.trim());
                            if p.exists() {
                                if let Some(d) = &self.current_note {
                                    match self.client.upload_attachment(d.id, &p) {
                                        Ok(_) => {
                                            self.set_status("Attachment uploaded", false);
                                            self.load_stats();
                                            self.load_current_note_detail();
                                        }
                                        Err(e) => {
                                            self.set_status(format!("Upload error: {}", e), true)
                                        }
                                    }
                                }
                            } else {
                                self.set_status("File path does not exist", true);
                            }
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                        KeyCode::Char(c) => {
                            path_str.push(c);
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: Some(path_str),
                                dest_input: None,
                            });
                        }
                        KeyCode::Backspace => {
                            path_str.pop();
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: Some(path_str),
                                dest_input: None,
                            });
                        }
                        _ => {}
                    }
                    return;
                }

                if let Some(mut save_str) = dest_input {
                    match key {
                        KeyCode::Esc => {
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                        KeyCode::Enter => {
                            if let Some(att) = attachments.get(cursor) {
                                let p = PathBuf::from(save_str.trim());
                                match self.client.download_attachment(&att.url, &p) {
                                    Ok(_) => self.set_status(
                                        format!("Downloaded attachment to {:?}", p),
                                        false,
                                    ),
                                    Err(e) => {
                                        self.set_status(format!("Download error: {}", e), true)
                                    }
                                }
                            }
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                        KeyCode::Char(c) => {
                            save_str.push(c);
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: Some(save_str),
                            });
                        }
                        KeyCode::Backspace => {
                            save_str.pop();
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: Some(save_str),
                            });
                        }
                        _ => {}
                    }
                    return;
                }

                match key {
                    KeyCode::Esc => self.modal = None,
                    KeyCode::Up | KeyCode::Char('k') => {
                        if cursor > 0 {
                            self.modal = Some(Modal::Attachments {
                                cursor: cursor - 1,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !attachments.is_empty() && cursor + 1 < attachments.len() {
                            self.modal = Some(Modal::Attachments {
                                cursor: cursor + 1,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                    }
                    KeyCode::Char('u') => {
                        self.modal = Some(Modal::Attachments {
                            cursor,
                            upload_input: Some(String::new()),
                            dest_input: None,
                        });
                    }
                    KeyCode::Char('s') => {
                        if let Some(att) = attachments.get(cursor) {
                            let default_dest = format!("./{}", att.filename);
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: Some(default_dest),
                            });
                        }
                    }
                    KeyCode::Char('y') => {
                        if let Some(att) = attachments.get(cursor) {
                            if let Some(cb) = &mut self.clipboard {
                                let _ = cb.set_text(att.url.clone());
                                self.set_status("Copied attachment URL", false);
                            }
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(att) = attachments.get(cursor) {
                            if let Some(d) = &self.current_note {
                                self.modal = Some(Modal::ConfirmDelete {
                                    target: DeleteTarget::Attachment {
                                        note_id: d.id,
                                        attachment_id: att.id,
                                        filename: att.filename.clone(),
                                    },
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }

            Some(Modal::Help) => match key {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') | KeyCode::Char('q') => {
                    self.modal = None;
                }
                _ => {}
            },

            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_note(id: u64, title: &str, pinned: bool) -> Note {
        Note {
            id,
            project_id: None,
            project_name: None,
            title: title.to_string(),
            body: None,
            pinned,
            pinned_position: None,
            attachments_count: 0,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn test_sort_notes_pinned_first_and_alphabetical() {
        let mut notes = vec![
            make_test_note(1, "zebra", false),
            make_test_note(2, "apple", false),
            make_test_note(3, "Banana", true),
            make_test_note(4, "avocado", true),
            make_test_note(5, "Carrot", false),
        ];

        sort_notes(&mut notes);

        let titles: Vec<&str> = notes.iter().map(|n| n.title.as_str()).collect();
        // Pinned first: avocado, Banana
        // Unpinned second: apple, Carrot, zebra
        assert_eq!(titles, vec!["avocado", "Banana", "apple", "Carrot", "zebra"]);
        assert!(notes[0].pinned);
        assert!(notes[1].pinned);
        assert!(!notes[2].pinned);
        assert!(!notes[3].pinned);
        assert!(!notes[4].pinned);
    }

    #[test]
    fn test_sidebar_items_resolution() {
        let config = Config::new("http://localhost:3000");
        let app = App {
            config: config.clone(),
            client: ApiClient::new(&config),
            should_quit: false,
            pending_editor: None,
            status: None,
            focused_pane: Pane::Sidebar,
            view_mode: ViewMode::Preview,
            modal: None,
            stats: Stats::default(),
            preset_hosts: Vec::new(),
            projects: vec![
                Project {
                    id: 10,
                    name: "Work".to_string(),
                    description: None,
                    notes_count: 3,
                    created_at: None,
                    updated_at: None,
                },
                Project {
                    id: 20,
                    name: "Home".to_string(),
                    description: None,
                    notes_count: 1,
                    created_at: None,
                    updated_at: None,
                },
            ],
            pinned_notes: vec![
                make_test_note(101, "Architecture", true),
                make_test_note(102, "Sprint Notes", true),
            ],
            notes: Vec::new(),
            current_note: None,
            checklist_items: Vec::new(),
            sidebar_selected: 0,
            notes_selected: 0,
            detail_selected_checklist: None,
            detail_scroll: 0,
            search_query: None,
            clipboard: None,
        };

        // 3 scopes + 2 projects + 2 pinned = 7 items
        assert_eq!(app.total_sidebar_items(), 7);

        assert_eq!(app.get_sidebar_item(0), ScopeItem::All);
        assert_eq!(app.get_sidebar_item(1), ScopeItem::Pinned);
        assert_eq!(app.get_sidebar_item(2), ScopeItem::Standalone);
        assert_eq!(
            app.get_sidebar_item(3),
            ScopeItem::Project(app.projects[0].clone())
        );
        assert_eq!(
            app.get_sidebar_item(4),
            ScopeItem::Project(app.projects[1].clone())
        );
        assert_eq!(
            app.get_sidebar_item(5),
            ScopeItem::PinnedNote(app.pinned_notes[0].clone())
        );
        assert_eq!(
            app.get_sidebar_item(6),
            ScopeItem::PinnedNote(app.pinned_notes[1].clone())
        );
        // Out of bounds falls back to All
        assert_eq!(app.get_sidebar_item(7), ScopeItem::All);
    }
}
