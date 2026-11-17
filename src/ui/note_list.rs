use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

use crate::app::{App, Pane, ScopeItem};
use crate::checklist::checklist_progress;
use crate::format::date_short;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == Pane::Notes;
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

    let scope_name = match app.get_sidebar_item(app.sidebar_selected) {
        ScopeItem::All => "All Notes".to_string(),
        ScopeItem::Pinned => "★ Pinned".to_string(),
        ScopeItem::Standalone => "Standalone".to_string(),
        ScopeItem::Project(p) => p.name,
        ScopeItem::PinnedNote(n) => format!("★ {}", n.title),
    };

    let title_text = format!(" [2] Notes ({} · {}) ", scope_name, app.notes.len());

    let mut items = Vec::new();

    if app.notes.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  (no notes found — press 'n' to create one)",
            Style::default().fg(Color::DarkGray),
        ))));
    } else {
        for (idx, note) in app.notes.iter().enumerate() {
            let is_selected = app.notes_selected == idx;
            let cursor = if is_selected { "▶ " } else { "  " };

            let star = if note.pinned { "★ " } else { "  " };
            let star_style = if note.pinned {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let item_title_style = if is_selected {
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Checklist progress if note contains tasks
            let (done, total) = checklist_progress(note.body.as_deref().unwrap_or(""));
            let progress_span = if total > 0 {
                let color = if done == total {
                    Color::Green
                } else if done > 0 {
                    Color::LightYellow
                } else {
                    Color::DarkGray
                };
                Span::styled(format!(" [{}/{}]", done, total), Style::default().fg(color))
            } else {
                Span::raw("")
            };

            // Attachments indicator
            let att_span = if note.attachments_count > 0 {
                Span::styled(
                    format!(" 📎{}", note.attachments_count),
                    Style::default().fg(Color::Blue),
                )
            } else {
                Span::raw("")
            };

            // Project badge
            let proj_span = if let Some(pname) = &note.project_name {
                Span::styled(
                    format!(" · {}", pname),
                    Style::default().fg(Color::Magenta),
                )
            } else {
                Span::raw("")
            };

            let date_str = note
                .updated_at
                .as_deref()
                .map(date_short)
                .unwrap_or_default();
            let date_span = Span::styled(
                format!("  {}", date_str),
                Style::default().fg(Color::DarkGray),
            );

            let line = Line::from(vec![
                Span::styled(cursor, item_title_style),
                Span::styled(star, star_style),
                Span::styled(note.title.clone(), item_title_style),
                progress_span,
                att_span,
                proj_span,
                date_span,
            ]);

            items.push(ListItem::new(line));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title_text, title_style));

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}
