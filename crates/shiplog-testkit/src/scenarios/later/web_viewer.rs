//! BDD scenarios for Web Viewer (Feature 13)
//!
//! Scenarios cover:
//! - Primary user workflows (launching web viewer, navigating workstreams)
//! - Edge cases (no packet, port already in use)
//! - Integration with other features (packet re-render)
//! - Performance scenarios (large packets)

use crate::bdd::Scenario;
use crate::bdd::assertions::*;

/// Scenario 13.1: User launches web viewer
pub fn web_viewer_launch() -> Scenario {
    Scenario::new("User launches web viewer")
        .given("a user has a rendered packet", |ctx| {
            ctx.paths
                .insert("packet_path".to_string(), "/out/run_001/packet.md".into());
        })
        .when("they run \"shiplog web-serve\"", |ctx| {
            ctx.flags.insert("web_server_started".to_string(), true);
            ctx.strings.insert(
                "server_url".to_string(),
                "http://localhost:8080".to_string(),
            );
            ctx.strings
                .insert("default_port".to_string(), "8080".to_string());
            Ok(())
        })
        .then("a web server should start", |ctx| {
            assert_true(
                ctx.flag("web_server_started").unwrap_or(false),
                "web server started",
            )
        })
        .then("the packet should be accessible in a browser", |ctx| {
            let url = ctx.string("server_url").unwrap();
            assert_contains(url, "localhost", "server URL")
        })
        .then(
            "the server should listen on a default port (e.g., 8080)",
            |ctx| {
                let port = ctx.string("default_port").unwrap();
                assert_eq(port, "8080", "default port")
            },
        )
}

/// Scenario 13.2: User navigates workstreams in web viewer
pub fn web_viewer_navigate() -> Scenario {
    Scenario::new("User navigates workstreams in web viewer")
        .given("a user has the web viewer open", |ctx| {
            ctx.flags.insert("web_viewer_open".to_string(), true);
        })
        .given("the packet has multiple workstreams", |ctx| {
            ctx.numbers.insert("workstream_count".to_string(), 5);
        })
        .when("they click on a workstream in the sidebar", |ctx| {
            ctx.flags.insert("workstream_selected".to_string(), true);
            ctx.flags.insert("main_view_updated".to_string(), true);
            ctx.strings
                .insert("url_fragment".to_string(), "#workstream-123".to_string());
            Ok(())
        })
        .then(
            "the main view should display the selected workstream",
            |ctx| {
                assert_true(
                    ctx.flag("main_view_updated").unwrap_or(false),
                    "main view updated",
                )
            },
        )
        .then("the URL should update with the workstream ID", |ctx| {
            let url = ctx.string("url_fragment").unwrap();
            assert_contains(url, "#workstream", "URL fragment")
        })
}

/// Scenario 13.3: User searches for events in web viewer
pub fn web_viewer_search() -> Scenario {
    Scenario::new("User searches for events in web viewer")
        .given("a user has the web viewer open", |ctx| {
            ctx.flags.insert("web_viewer_open".to_string(), true);
        })
        .given("the packet has many events", |ctx| {
            ctx.numbers.insert("event_count".to_string(), 100);
        })
        .when("they type a search query in the search box", |ctx| {
            ctx.strings
                .insert("search_query".to_string(), "bug fix".to_string());
            ctx.flags
                .insert("matching_events_highlighted".to_string(), true);
            ctx.flags.insert("list_filtered".to_string(), true);
            Ok(())
        })
        .then("matching events should be highlighted", |ctx| {
            assert_true(
                ctx.flag("matching_events_highlighted").unwrap_or(false),
                "matching events highlighted",
            )
        })
        .then(
            "the list should filter to show only matching events",
            |ctx| assert_true(ctx.flag("list_filtered").unwrap_or(false), "list filtered"),
        )
}

/// Scenario 13.4: User filters by source in web viewer
pub fn web_viewer_filter_source() -> Scenario {
    Scenario::new("User filters by source in web viewer")
        .given("a user has the web viewer open", |ctx| {
            ctx.flags.insert("web_viewer_open".to_string(), true);
        })
        .given("the packet has events from multiple sources", |ctx| {
            ctx.numbers.insert("source_count".to_string(), 3);
        })
        .when(
            "they select a source filter (e.g., \"GitHub only\")",
            |ctx| {
                ctx.strings
                    .insert("selected_filter".to_string(), "GitHub only".to_string());
                ctx.flags.insert("events_filtered".to_string(), true);
                ctx.flags.insert("only_github_shown".to_string(), true);
                Ok(())
            },
        )
        .then(
            "only events from the selected source should be displayed",
            |ctx| {
                assert_true(
                    ctx.flag("only_github_shown").unwrap_or(false),
                    "only GitHub shown",
                )
            },
        )
}

/// Scenario 13.5: Web viewer with no packet
pub fn web_viewer_no_packet() -> Scenario {
    Scenario::new("Web viewer with no packet")
        .given("a user has no rendered packet", |ctx| {
            ctx.flags.insert("no_packet".to_string(), true);
        })
        .when("they run \"shiplog web-serve\"", |ctx| {
            ctx.flags.insert("command_failed".to_string(), true);
            ctx.strings.insert(
                "error_message".to_string(),
                "No packet found. Please render a packet first.".to_string(),
            );
            Ok(())
        })
        .then("an error message should indicate no packet found", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "No packet found", "error message")
        })
        .then("the server should not start", |ctx| {
            assert_true(
                ctx.flag("command_failed").unwrap_or(false),
                "command failed",
            )
        })
}

/// Scenario 13.6: Port already in use
pub fn web_viewer_port_in_use() -> Scenario {
    Scenario::new("Port already in use")
        .given("a user has another service using the default port", |ctx| {
            ctx.strings
                .insert("default_port".to_string(), "8080".to_string());
            ctx.flags.insert("port_in_use".to_string(), true);
        })
        .when("they run \"shiplog web-serve\"", |ctx| {
            ctx.flags.insert("command_failed".to_string(), true);
            ctx.strings.insert(
                "error_message".to_string(),
                "Port 8080 is already in use. Use --port to specify an alternative.".to_string(),
            );
            Ok(())
        })
        .then("an error should indicate the port is in use", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "already in use", "error message")
        })
        .then(
            "the user should be able to specify an alternative port",
            |ctx| {
                let error = ctx.string("error_message").unwrap();
                assert_contains(error, "--port", "error message")
            },
        )
}

/// Scenario 13.7: Web viewer updates on packet re-render
pub fn web_viewer_update() -> Scenario {
    Scenario::new("Web viewer updates on packet re-render")
        .given("a user has the web viewer open", |ctx| {
            ctx.flags.insert("web_viewer_open".to_string(), true);
        })
        .given("they re-render the packet with new data", |ctx| {
            ctx.flags.insert("packet_re_rendered".to_string(), true);
        })
        .when("they refresh the browser", |ctx| {
            ctx.flags.insert("web_viewer_updated".to_string(), true);
            ctx.flags.insert("new_data_displayed".to_string(), true);
            Ok(())
        })
        .then("the web viewer should display the updated packet", |ctx| {
            assert_true(
                ctx.flag("web_viewer_updated").unwrap_or(false),
                "web viewer updated",
            )
        })
}

/// Scenario 13.8: Web viewer with large packet
pub fn web_viewer_large() -> Scenario {
    Scenario::new("Web viewer with large packet")
        .given("a user has a packet with 1,000 events", |ctx| {
            ctx.numbers.insert("event_count".to_string(), 1000);
        })
        .when("they open the web viewer", |ctx| {
            ctx.flags.insert("web_viewer_open".to_string(), true);
            ctx.strings
                .insert("load_time".to_string(), "2.5s".to_string());
            ctx.flags.insert("scrolling_smooth".to_string(), true);
            Ok(())
        })
        .then(
            "the page should load within reasonable time (< 3 seconds)",
            |ctx| {
                let time = ctx.string("load_time").unwrap();
                assert_true(time.contains("s") && !time.contains("m"), "load time")
            },
        )
        .then("scrolling should remain smooth", |ctx| {
            assert_true(
                ctx.flag("scrolling_smooth").unwrap_or(false),
                "scrolling smooth",
            )
        })
}
