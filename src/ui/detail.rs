use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Pane, ViewMode};
use crate::checklist::ChecklistItem;
use crate::format::{datetime_compact, word_and_line_count};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == Pane::Detail;
    let border_color = if is_focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let title_style = if is_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let block_title = format!(" [3] Note Detail [{}] ", app.view_mode.label());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(block_title, title_style));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let detail = match &app.current_note {
        Some(d) => d,
        None => {
            let empty = Paragraph::new(Line::from(Span::styled(
                "No note selected",
                Style::default().fg(Color::DarkGray),
            )));
            f.render_widget(empty, inner);
            return;
        }
    };

    // Split inner area into: Header (metadata: 3 lines), Divider (1 line), Body (Min(0))
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    // Metadata lines
    let mut meta_lines = Vec::new();
    let star = if detail.pinned { " ★ Pinned" } else { "" };
    let proj_name = detail
        .project_name
        .as_deref()
        .unwrap_or("Standalone (scratchpad)");

    meta_lines.push(Line::from(vec![
        Span::styled(
            &detail.title,
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            star,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    let body_text = detail.body.as_deref().unwrap_or("");
    let (words, lines_count) = word_and_line_count(body_text);

    let updated = detail
        .updated_at
        .as_deref()
        .map(datetime_compact)
        .unwrap_or_else(|| "unknown".into());

    let att_text = if detail.attachments_count > 0 {
        format!(" · 📎 {} files ('A' to view)", detail.attachments_count)
    } else {
        " · 📎 0 files ('A' to attach)".to_string()
    };

    meta_lines.push(Line::from(vec![
        Span::styled(
            format!("Project: {}", proj_name),
            Style::default().fg(Color::Magenta),
        ),
        Span::styled(
            format!(" · {}w / {}L", words, lines_count),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!(" · Updated: {}", updated),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(att_text, Style::default().fg(Color::Blue)),
    ]));

    f.render_widget(
        Paragraph::new(meta_lines).wrap(Wrap { trim: false }),
        chunks[0],
    );

    // Divider
    let divider = Paragraph::new(Line::from(Span::styled(
        "─".repeat(chunks[1].width as usize),
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(divider, chunks[1]);

    // Body rendering according to ViewMode
    let rendered_lines = match app.view_mode {
        ViewMode::Preview => render_markdown_preview(
            body_text,
            &app.checklist_items,
            app.detail_selected_checklist,
            is_focused,
            chunks[2].width as usize,
        ),
        ViewMode::Raw => render_markdown_raw(body_text),
    };

    let scroll_y = app.detail_scroll as u16;
    let body_paragraph = Paragraph::new(rendered_lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll_y, 0));
    f.render_widget(body_paragraph, chunks[2]);
}

fn render_markdown_raw(text: &str) -> Vec<Line<'_>> {
    if text.trim().is_empty() {
        return vec![Line::from(Span::styled(
            "(empty note body — press 'a' to append content, 'E' to edit in $EDITOR)",
            Style::default().fg(Color::DarkGray),
        ))];
    }

    text.lines()
        .enumerate()
        .map(|(idx, raw_line)| {
            Line::from(vec![
                Span::styled(
                    format!("{:>4} │ ", idx + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(raw_line, Style::default().fg(Color::White)),
            ])
        })
        .collect()
}

fn render_markdown_preview<'a>(
    text: &'a str,
    checklist_items: &[ChecklistItem],
    selected_checklist: Option<usize>,
    pane_focused: bool,
    width: usize,
) -> Vec<Line<'a>> {
    if text.trim().is_empty() {
        return vec![Line::from(Span::styled(
            "(empty note body — press 'a' to append content, 'E' to edit in $EDITOR)",
            Style::default().fg(Color::DarkGray),
        ))];
    }

    let mut out = Vec::new();
    let mut in_code_block = false;

    for (line_idx_0, raw_line) in text.lines().enumerate() {
        let line_num = line_idx_0 + 1;
        let trimmed = raw_line.trim_start();

        // Fenced code blocks
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            out.push(Line::from(Span::styled(
                raw_line,
                Style::default().fg(Color::DarkGray),
            )));
            continue;
        }

        if in_code_block {
            out.push(Line::from(Span::styled(
                format!("  {}", raw_line),
                Style::default().fg(Color::Green),
            )));
            continue;
        }

        // Check if this line is one of our parsed checklist items
        if let Some(item) = checklist_items.iter().find(|i| i.line_number == line_num) {
            let is_selected_item = selected_checklist == Some(item.index);
            let bullet_cursor = if is_selected_item && pane_focused {
                "▶ "
            } else {
                "  "
            };

            let mark_span = if item.completed {
                Span::styled(
                    "[x] ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    "[ ] ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            };

            let text_style = if is_selected_item && pane_focused {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else if item.completed {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default().fg(Color::White)
            };

            let cursor_style = if is_selected_item && pane_focused {
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let line = Line::from(vec![
                Span::styled(bullet_cursor, cursor_style),
                Span::styled(item.indent.clone(), Style::default()),
                mark_span,
                Span::styled(item.text.clone(), text_style),
            ]);

            out.push(line);
            continue;
        }

        // Horizontal rules
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            out.push(Line::from(Span::styled(
                "─".repeat(width.saturating_sub(2).max(10)),
                Style::default().fg(Color::DarkGray),
            )));
            continue;
        }

        // Headings
        if trimmed.starts_with("# ") {
            out.push(Line::from(Span::styled(
                raw_line,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
        } else if trimmed.starts_with("## ") {
            out.push(Line::from(Span::styled(
                raw_line,
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if trimmed.starts_with("### ") || trimmed.starts_with("#### ") {
            out.push(Line::from(Span::styled(
                raw_line,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if trimmed.starts_with('>') {
            out.push(Line::from(Span::styled(
                raw_line,
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::ITALIC),
            )));
        } else if trimmed.starts_with('|') && trimmed.ends_with('|') {
            // Markdown table rows
            out.push(Line::from(Span::styled(
                raw_line,
                Style::default().fg(Color::LightCyan),
            )));
        } else if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* "))
        {
            let indent = &raw_line[..(raw_line.len() - trimmed.len())];
            out.push(Line::from(vec![
                Span::raw(indent),
                Span::styled("• ", Style::default().fg(Color::Yellow)),
                Span::styled(rest, Style::default().fg(Color::White)),
            ]));
        } else {
            out.push(Line::from(Span::styled(
                raw_line,
                Style::default().fg(Color::White),
            )));
        }
    }

    out
}
