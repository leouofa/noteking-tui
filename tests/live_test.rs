use noteking_tui::api::ApiClient;
use noteking_tui::app::{App, ViewMode};
use noteking_tui::checklist::checklist_progress;
use noteking_tui::config::Config;
use noteking_tui::ui;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn test_live_instance_and_ui_render() {
    let test_url = std::env::var("NOTEKING_TEST_URL")
        .or_else(|_| std::env::var("NOTEKING_REMOTE_URL"))
        .unwrap_or_else(|_| "http://100.96.219.81:3009".to_string());

    let config = Config::new(&test_url);
    let client = ApiClient::new(&config);

    if let Ok(stats) = client.get_stats() {
        println!(
            "Connected to NoteKing at {}: {} notes ({} pinned, {} standalone), {} projects, {} attachments",
            test_url,
            stats.total_notes,
            stats.pinned_notes,
            stats.standalone_notes,
            stats.total_projects,
            stats.total_attachments
        );
        if let Ok(notes) = client.list_notes(None, None, None, None) {
            if let Some(first) = notes.first() {
                if let Ok(detail) = client.get_note(first.id) {
                    let (done, total) = checklist_progress(detail.body.as_deref().unwrap_or(""));
                    println!(
                        "Checked Note '{}': {} checklist items ({} completed)",
                        detail.title, total, done
                    );
                }
            }
        }

        // Verify full TUI rendering in both Preview and Raw modes
        let mut app = App::new(config);
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui::render(f, &app)).unwrap();
        app.view_mode = ViewMode::Raw;
        terminal.draw(|f| ui::render(f, &app)).unwrap();
    } else {
        println!(
            "Live server at {} not reachable during test run (set NOTEKING_TEST_URL to override)",
            test_url
        );
    }
}
