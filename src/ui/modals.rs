use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, DeleteTarget, Modal};
use crate::format::format_bytes;

pub fn render(f: &mut Frame, app: &App) {
    let modal = match &app.modal {
        Some(m) => m,
        None => return,
    };

    match modal {
        Modal::HostSwitcher {
            cursor,
            custom_input,
            entering_custom,
        } => render_host_switcher(f, app, *cursor, custom_input, *entering_custom),
        Modal::Search { input } => render_search(f, input),
        Modal::QuickAppendNote {
            content_input,
            section_input,
            as_task,
            focus_field,
        } => render_quick_append(f, content_input, section_input, *as_task, *focus_field),
        Modal::ProjectForm {
            id,
            name_input,
            desc_input,
            focus_desc,
        } => render_project_form(f, id.is_some(), name_input, desc_input, *focus_desc),
        Modal::NoteForm {
            id,
            title_input,
            project_choice_idx,
            pinned,
            focus_field,
        } => render_note_form(
            f,
            app,
            id.is_some(),
            title_input,
            *project_choice_idx,
            *pinned,
            *focus_field,
        ),
        Modal::ConfirmDelete { target } => render_confirm_delete(f, target),
        Modal::Attachments {
            cursor,
            upload_input,
            dest_input,
        } => render_attachments(
            f,
            app,
            *cursor,
            upload_input.as_deref(),
            dest_input.as_deref(),
        ),
        Modal::Help => render_help(f, app.config.allow_host_switcher),
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_host_switcher(
    f: &mut Frame,
    app: &App,
    cursor: usize,
    custom_input: &str,
    entering_custom: bool,
) {
    let area = centered_rect(60, 45, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Switch NoteKing Host URL ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("Current Active Host: {}", app.config.base_url),
        Style::default()
            .fg(Color::LightYellow)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Preset Hosts:",
        Style::default().fg(Color::Gray),
    )));

    for (i, preset) in app.preset_hosts.iter().enumerate() {
        let is_sel = !entering_custom && cursor == i;
        let prefix = if is_sel { " ▶ " } else { "   " };
        let style = if is_sel {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if *preset == app.config.base_url {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };
        let active_mark = if *preset == app.config.base_url {
            " (active)"
        } else {
            ""
        };
        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(preset.as_str(), style),
            Span::styled(active_mark, Style::default().fg(Color::Green)),
        ]));
    }

    lines.push(Line::from(""));
    let custom_sel = entering_custom || cursor == app.preset_hosts.len();
    let custom_prefix = if custom_sel {
        " ▶ Custom URL: "
    } else {
        "   Custom URL: "
    };
    let custom_style = if custom_sel {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    lines.push(Line::from(vec![
        Span::styled(custom_prefix, custom_style),
        Span::styled(
            if custom_input.is_empty() {
                "(type URL e.g. http://100.96.219.81:3009)"
            } else {
                custom_input
            },
            if custom_input.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            },
        ),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Keys: ↑/↓: select preset | Tab: toggle custom edit | Enter: apply | Esc: cancel",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_search(f: &mut Frame, input: &str) {
    let area = centered_rect(50, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Search Notes ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled(
            "Filter query (title or body):",
            Style::default().fg(Color::Gray),
        )),
        Line::from(vec![
            Span::styled(" / ", Style::default().fg(Color::Yellow)),
            Span::styled(
                if input.is_empty() {
                    "(type keyword...)"
                } else {
                    input
                },
                if input.is_empty() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                },
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Enter: apply filter | Esc: clear & cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_quick_append(
    f: &mut Frame,
    content_input: &str,
    section_input: &str,
    as_task: bool,
    focus_field: usize,
) {
    let area = centered_rect(64, 36, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Quick Append to Note ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content_style = if focus_field == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let sec_style = if focus_field == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let task_style = if focus_field == 2 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let prefix_str = if as_task { " - [ ] " } else { " > " };

    let lines = vec![
        Line::from(Span::styled("Markdown content / log entry:", content_style)),
        Line::from(vec![
            Span::styled(prefix_str, Style::default().fg(Color::Green)),
            Span::styled(
                if content_input.is_empty() {
                    "(enter note text or task)"
                } else {
                    content_input
                },
                if content_input.is_empty() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    content_style
                },
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("Section header (optional):", sec_style)),
        Line::from(vec![
            Span::styled(" ## ", Style::default().fg(Color::Cyan)),
            Span::styled(
                if section_input.is_empty() {
                    "(leave blank to append to end of note)"
                } else {
                    section_input
                },
                if section_input.is_empty() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    sec_style
                },
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("Format (Space to toggle):", task_style)),
        Line::from(vec![Span::styled(
            if as_task {
                " [x] Format as checklist task (- [ ] ...)"
            } else {
                " [ ] Standard Markdown paragraph / note entry"
            },
            if as_task {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            },
        )]),
        Line::from(""),
        Line::from(Span::styled(
            "Tab: switch field | Enter: append to note | Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_project_form(f: &mut Frame, is_edit: bool, name: &str, desc: &str, focus_desc: bool) {
    let area = centered_rect(60, 32, f.area());
    f.render_widget(Clear, area);

    let title = if is_edit {
        " Edit Project "
    } else {
        " New Project "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let name_style = if !focus_desc {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let desc_style = if focus_desc {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let lines = vec![
        Line::from(Span::styled("Project Name:", name_style)),
        Line::from(vec![
            Span::styled(" > ", Style::default().fg(Color::Magenta)),
            Span::styled(
                if name.is_empty() { "(required)" } else { name },
                if name.is_empty() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    name_style
                },
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("Description (optional):", desc_style)),
        Line::from(vec![
            Span::styled(" > ", Style::default().fg(Color::Magenta)),
            Span::styled(
                if desc.is_empty() { "(optional)" } else { desc },
                if desc.is_empty() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    desc_style
                },
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Tab: switch field | Enter: save | Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_note_form(
    f: &mut Frame,
    app: &App,
    is_edit: bool,
    title_input: &str,
    project_idx: usize,
    pinned: bool,
    focus_field: usize,
) {
    let area = centered_rect(65, 36, f.area());
    f.render_widget(Clear, area);

    let title = if is_edit {
        " Edit Note Metadata "
    } else {
        " New Note "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let title_style = if focus_field == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let proj_style = if focus_field == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let pin_style = if focus_field == 2 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let proj_label = if project_idx == 0 {
        "None (Standalone Scratchpad)".to_string()
    } else if let Some(p) = app.projects.get(project_idx - 1) {
        p.name.clone()
    } else {
        "None".to_string()
    };

    let lines = vec![
        Line::from(Span::styled("Title:", title_style)),
        Line::from(vec![
            Span::styled(" > ", Style::default().fg(Color::Yellow)),
            Span::styled(
                if title_input.is_empty() {
                    "(enter note title)"
                } else {
                    title_input
                },
                if title_input.is_empty() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    title_style
                },
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("Project (use ←/→ to change):", proj_style)),
        Line::from(vec![
            Span::styled(" [◄ ", Style::default().fg(Color::Magenta)),
            Span::styled(proj_label, proj_style),
            Span::styled(" ►]", Style::default().fg(Color::Magenta)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Pin status (Space to toggle):", pin_style)),
        Line::from(vec![Span::styled(
            if pinned {
                " [x] ★ Pinned"
            } else {
                " [ ] Unpinned"
            },
            if pinned {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            },
        )]),
        Line::from(""),
        Line::from(Span::styled(
            "Tab: switch field | Enter: save | Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_confirm_delete(f: &mut Frame, target: &DeleteTarget) {
    let area = centered_rect(50, 22, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Confirm Deletion ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let desc = match target {
        DeleteTarget::Project(p) => format!(
            "Project: \"{}\" (notes preserved as standalone)",
            p.name
        ),
        DeleteTarget::Note(n) => format!("Note: \"{}\"", n.title),
        DeleteTarget::Attachment { filename, .. } => format!("Attachment: \"{}\"", filename),
    };

    let lines = vec![
        Line::from(Span::styled(
            "Are you sure you want to permanently delete this?",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(desc, Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(Span::styled(
            "y / Enter: delete | n / Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_attachments(
    f: &mut Frame,
    app: &App,
    cursor: usize,
    upload_input: Option<&str>,
    dest_input: Option<&str>,
) {
    let area = centered_rect(70, 50, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Note Context Attachments ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(upload_path) = upload_input {
        let lines = vec![
            Line::from(Span::styled(
                "Upload File to Current Note:",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(Color::White)),
                Span::styled(
                    if upload_path.is_empty() {
                        "(type absolute or relative file path...)"
                    } else {
                        upload_path
                    },
                    if upload_path.is_empty() {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    },
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Enter: upload | Esc: cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }

    if let Some(dest_path) = dest_input {
        let lines = vec![
            Line::from(Span::styled(
                "Save Attachment to Disk:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Destination: ", Style::default().fg(Color::White)),
                Span::styled(
                    dest_path,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Enter: download | Esc: cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }

    let attachments = app
        .current_note
        .as_ref()
        .map(|d| d.attachments.clone())
        .unwrap_or_default();

    let mut items = Vec::new();

    if attachments.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  (no attachments on this note — press 'u' to upload a file)",
            Style::default().fg(Color::DarkGray),
        ))));
    } else {
        for (i, att) in attachments.iter().enumerate() {
            let is_sel = cursor == i;
            let prefix = if is_sel { " ▶ " } else { "   " };
            let style = if is_sel {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let size_str = format_bytes(att.byte_size);
            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("{:<30} ", att.filename), style),
                Span::styled(
                    format!("{:>10} ", size_str),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    att.content_type.as_deref().unwrap_or("unknown"),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);

    f.render_widget(List::new(items), layout[0]);

    let footer = Paragraph::new(Line::from(Span::styled(
        "Keys: j/k: select | u: upload file | s: save to disk | y: copy URL | d: delete | Esc: close",
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(footer, layout[1]);
}

fn render_help(f: &mut Frame, allow_switch: bool) {
    let area = centered_rect(76, 78, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" NoteKing-TUI Keyboard Shortcuts & Help ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let host_line = if allow_switch {
        Line::from("  H / u          Switch Host URL presets or enter custom URL")
    } else {
        Line::from(
            "  H / u          [Locked by NOTEKING_REMOTE_URL] (pass --allow-switch to enable)",
        )
    };

    let lines = vec![
        Line::from(Span::styled(
            "Navigation & Focus",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  1, 2, 3        Direct focus to pane 1 (Sidebar), 2 (Notes), 3 (Detail)"),
        Line::from("  Tab / BackTab  Cycle focus forward / backward between panes"),
        Line::from("  h / l          Move focus left / right"),
        Line::from("  j / k          Move selection down / up in focused pane"),
        Line::from("  Enter          Enter pane 2 from sidebar, or pane 3 from notes list"),
        Line::from(""),
        Line::from(Span::styled(
            "Note Detail, Markdown & Checklists (Pane 3)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  v / m          Toggle between Styled Preview and Raw Markdown (line numbers)"),
        Line::from("  j / k          Scroll note (or navigate checklist items when present)"),
        Line::from("  Space / Enter  Toggle active checklist item [ ] ⇄ [x] (if note has tasks)"),
        Line::from("  a              Quick-append Markdown section, log entry, or task to note"),
        Line::from("  E              Open note Markdown body in external $EDITOR (nvim/vim/nano)"),
        Line::from("  J / K          Scroll detail document view down / up by 3 lines"),
        Line::from("  g / G          Jump to top / bottom of note document"),
        Line::from("  c / Y          Copy entire Markdown body to system clipboard"),
        Line::from("  y              Yank active task text (or note title) to system clipboard"),
        Line::from(""),
        Line::from(Span::styled(
            "Workspace Management & Modals",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        host_line,
        Line::from("  /              Search & filter notes by title or body in real-time"),
        Line::from("  n              Create New Project (in pane 1) or Note (in pane 2/3)"),
        Line::from("  e              Edit metadata of selected Project or Note"),
        Line::from("  d              Delete selected Project, Note, or Attachment"),
        Line::from("  p              Toggle pinned status of selected Note"),
        Line::from("  K / J          Reorder pinned notes up / down (in Pinned scope)"),
        Line::from("  A              Open attachments manager modal (upload, download, delete)"),
        Line::from("  r              Refresh all data from NoteKing server"),
        Line::from("  ?              Open this help dialog"),
        Line::from("  q              Quit application (or Esc to close dialogs)"),
    ];

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
