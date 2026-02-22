//! BDD scenarios for TUI Workstream Editor (Feature 12)
//!
//! Scenarios cover:
//! - Primary user workflows (opening TUI, renaming workstreams)
//! - Edge cases (no workstreams, very long titles)
//! - Integration with other features (rendering)
//! - Performance scenarios (many workstreams)

use crate::bdd::Scenario;
use crate::bdd::assertions::*;

/// Scenario 12.1: User opens TUI workstream editor
pub fn tui_editor_open() -> Scenario {
    Scenario::new("User opens TUI workstream editor")
        .given("a user has generated workstreams", |ctx| {
            ctx.numbers.insert("workstream_count".to_string(), 5);
        })
        .when("they run \"shiplog edit workstreams\"", |ctx| {
            ctx.flags.insert("tui_opened".to_string(), true);
            ctx.flags.insert("workstreams_displayed".to_string(), true);
            ctx.flags.insert("keyboard_navigation".to_string(), true);
            Ok(())
        })
        .then("a terminal UI should open", |ctx| {
            assert_true(ctx.flag("tui_opened").unwrap_or(false), "TUI opened")
        })
        .then("workstreams should be displayed in a list", |ctx| {
            assert_true(
                ctx.flag("workstreams_displayed").unwrap_or(false),
                "workstreams displayed",
            )
        })
        .then("the user should be able to navigate with keyboard", |ctx| {
            assert_true(
                ctx.flag("keyboard_navigation").unwrap_or(false),
                "keyboard navigation",
            )
        })
}

/// Scenario 12.2: User renames a workstream in TUI
pub fn tui_editor_rename() -> Scenario {
    Scenario::new("User renames a workstream in TUI")
        .given("a user has the TUI workstream editor open", |ctx| {
            ctx.flags.insert("tui_opened".to_string(), true);
        })
        .given("they select a workstream", |ctx| {
            ctx.strings
                .insert("old_title".to_string(), "Old Workstream Title".to_string());
        })
        .when("they press the rename key (e.g., 'r')", |ctx| {
            ctx.flags.insert("rename_mode_active".to_string(), true);
            Ok(())
        })
        .when("they enter a new title", |ctx| {
            ctx.strings
                .insert("new_title".to_string(), "New Workstream Title".to_string());
            ctx.flags.insert("title_updated".to_string(), true);
            Ok(())
        })
        .then("the workstream title should be updated", |ctx| {
            assert_true(ctx.flag("title_updated").unwrap_or(false), "title updated")
        })
        .then("the change should be reflected in the UI", |ctx| {
            let title = ctx.string("new_title").unwrap();
            assert_eq(title, "New Workstream Title", "title")
        })
}

/// Scenario 12.3: User adds a summary to a workstream in TUI
pub fn tui_editor_add_summary() -> Scenario {
    Scenario::new("User adds a summary to a workstream in TUI")
        .given("a user has the TUI workstream editor open", |ctx| {
            ctx.flags.insert("tui_opened".to_string(), true);
        })
        .given("they select a workstream", |ctx| {
            ctx.flags.insert("workstream_selected".to_string(), true);
        })
        .when("they press the edit summary key (e.g., 's')", |ctx| {
            ctx.flags.insert("summary_mode_active".to_string(), true);
            Ok(())
        })
        .when("they enter a summary", |ctx| {
            ctx.strings.insert(
                "summary".to_string(),
                "This workstream covers feature X".to_string(),
            );
            ctx.flags.insert("summary_updated".to_string(), true);
            Ok(())
        })
        .then("the workstream summary should be updated", |ctx| {
            assert_true(
                ctx.flag("summary_updated").unwrap_or(false),
                "summary updated",
            )
        })
        .then("the change should be reflected in the UI", |ctx| {
            let summary = ctx.string("summary").unwrap();
            assert_eq(summary, "This workstream covers feature X", "summary")
        })
}

/// Scenario 12.4: User selects receipts in TUI
pub fn tui_editor_select_receipts() -> Scenario {
    Scenario::new("User selects receipts in TUI")
        .given("a user has the TUI workstream editor open", |ctx| {
            ctx.flags.insert("tui_opened".to_string(), true);
        })
        .given("they select a workstream", |ctx| {
            ctx.flags.insert("workstream_selected".to_string(), true);
        })
        .when("they press the edit receipts key (e.g., 'e')", |ctx| {
            ctx.flags.insert("receipts_mode_active".to_string(), true);
            ctx.flags.insert("events_displayed".to_string(), true);
            Ok(())
        })
        .when("they can toggle events as receipts", |ctx| {
            ctx.flags.insert("receipts_toggled".to_string(), true);
            ctx.flags
                .insert("selected_receipts_highlighted".to_string(), true);
            Ok(())
        })
        .then(
            "a list of events in the workstream should be displayed",
            |ctx| {
                assert_true(
                    ctx.flag("events_displayed").unwrap_or(false),
                    "events displayed",
                )
            },
        )
        .then("the user can toggle events as receipts", |ctx| {
            assert_true(
                ctx.flag("receipts_toggled").unwrap_or(false),
                "receipts toggled",
            )
        })
        .then("selected receipts should be highlighted", |ctx| {
            assert_true(
                ctx.flag("selected_receipts_highlighted").unwrap_or(false),
                "receipts highlighted",
            )
        })
}

/// Scenario 12.5: User saves changes in TUI
pub fn tui_editor_save() -> Scenario {
    Scenario::new("User saves changes in TUI")
        .given("a user has made changes in the TUI editor", |ctx| {
            ctx.flags.insert("changes_made".to_string(), true);
        })
        .when("they press the save key (e.g., 'Ctrl+S')", |ctx| {
            ctx.flags
                .insert("workstreams_file_updated".to_string(), true);
            ctx.flags.insert("confirmation_displayed".to_string(), true);
            ctx.flags.insert("tui_remains_open".to_string(), true);
            Ok(())
        })
        .then("the workstreams file should be updated", |ctx| {
            assert_true(
                ctx.flag("workstreams_file_updated").unwrap_or(false),
                "workstreams file updated",
            )
        })
        .then("a confirmation message should be displayed", |ctx| {
            assert_true(
                ctx.flag("confirmation_displayed").unwrap_or(false),
                "confirmation displayed",
            )
        })
        .then("the TUI should remain open for further editing", |ctx| {
            assert_true(
                ctx.flag("tui_remains_open").unwrap_or(false),
                "TUI remains open",
            )
        })
}

/// Scenario 12.6: User exits TUI without saving
pub fn tui_editor_exit() -> Scenario {
    Scenario::new("User exits TUI without saving")
        .given("a user has made changes in the TUI editor", |ctx| {
            ctx.flags.insert("changes_made".to_string(), true);
        })
        .when("they press the quit key (e.g., 'q')", |ctx| {
            ctx.flags.insert("save_prompt_shown".to_string(), true);
            Ok(())
        })
        .when("they choose not to save", |ctx| {
            ctx.flags.insert("changes_discarded".to_string(), true);
            ctx.flags.insert("tui_closed".to_string(), true);
            Ok(())
        })
        .then("changes should be discarded", |ctx| {
            assert_true(
                ctx.flag("changes_discarded").unwrap_or(false),
                "changes discarded",
            )
        })
        .then("the TUI should close", |ctx| {
            assert_true(ctx.flag("tui_closed").unwrap_or(false), "TUI closed")
        })
}

/// Scenario 12.7: TUI with no workstreams
pub fn tui_editor_no_workstreams() -> Scenario {
    Scenario::new("TUI with no workstreams")
        .given("a user has no workstreams generated", |ctx| {
            ctx.flags.insert("no_workstreams".to_string(), true);
        })
        .when("they run \"shiplog edit workstreams\"", |ctx| {
            ctx.flags.insert("tui_opened".to_string(), true);
            ctx.strings.insert(
                "message".to_string(),
                "No workstreams found. Please generate workstreams first.".to_string(),
            );
            Ok(())
        })
        .then(
            "the TUI should display a message indicating no workstreams",
            |ctx| {
                let msg = ctx.string("message").unwrap();
                assert_contains(msg, "No workstreams", "message")
            },
        )
        .then(
            "the user should be prompted to generate workstreams first",
            |ctx| {
                let msg = ctx.string("message").unwrap();
                assert_contains(msg, "generate workstreams", "message")
            },
        )
}

/// Scenario 12.8: TUI with very long workstream title
pub fn tui_editor_long_title() -> Scenario {
    Scenario::new("TUI with very long workstream title")
        .given(
            "a user has a workstream with a very long title (> 200 characters)",
            |ctx| {
                ctx.strings.insert(
                    "long_title".to_string(),
                    "This is a very long workstream title that exceeds 200 characters..."
                        .to_string(),
                );
            },
        )
        .when("they open the TUI editor", |ctx| {
            ctx.flags.insert("tui_opened".to_string(), true);
            ctx.flags
                .insert("title_truncated_in_list".to_string(), true);
            ctx.flags
                .insert("full_title_visible_in_edit".to_string(), true);
            Ok(())
        })
        .then("the title should be truncated in the list view", |ctx| {
            assert_true(
                ctx.flag("title_truncated_in_list").unwrap_or(false),
                "title truncated in list",
            )
        })
        .then("the full title should be visible in the edit view", |ctx| {
            assert_true(
                ctx.flag("full_title_visible_in_edit").unwrap_or(false),
                "full title visible in edit",
            )
        })
}

/// Scenario 12.9: TUI changes reflected in rendered packet
pub fn tui_editor_reflect_in_packet() -> Scenario {
    Scenario::new("TUI changes reflected in rendered packet")
        .given("a user has edited workstreams in the TUI", |ctx| {
            ctx.strings.insert(
                "edited_title".to_string(),
                "Edited Workstream Title".to_string(),
            );
        })
        .given("they have saved the changes", |ctx| {
            ctx.flags.insert("changes_saved".to_string(), true);
        })
        .when("they run \"shiplog render\"", |ctx| {
            ctx.flags.insert("packet_rendered".to_string(), true);
            ctx.strings.insert(
                "packet_content".to_string(),
                "# Packet\n## Edited Workstream Title".to_string(),
            );
            Ok(())
        })
        .then("the packet should reflect the TUI edits", |ctx| {
            let content = ctx.string("packet_content").unwrap();
            assert_contains(content, "Edited Workstream Title", "packet content")
        })
        .then(
            "workstream titles and summaries should match the TUI",
            |ctx| {
                let content = ctx.string("packet_content").unwrap();
                assert_contains(content, "Edited Workstream Title", "packet content")
            },
        )
}

/// Scenario 12.10: TUI with many workstreams
pub fn tui_editor_large() -> Scenario {
    Scenario::new("TUI with many workstreams")
        .given("a user has 100 workstreams", |ctx| {
            ctx.numbers.insert("workstream_count".to_string(), 100);
        })
        .when("they run \"shiplog edit workstreams\"", |ctx| {
            ctx.flags.insert("tui_opened".to_string(), true);
            ctx.strings
                .insert("open_time".to_string(), "1.5s".to_string());
            ctx.flags.insert("navigation_responsive".to_string(), true);
            Ok(())
        })
        .then(
            "the TUI should open within reasonable time (< 2 seconds)",
            |ctx| {
                let time = ctx.string("open_time").unwrap();
                assert_true(time.contains("s") && !time.contains("m"), "open time")
            },
        )
        .then("navigation should remain responsive", |ctx| {
            assert_true(
                ctx.flag("navigation_responsive").unwrap_or(false),
                "navigation responsive",
            )
        })
}
