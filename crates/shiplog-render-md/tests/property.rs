use proptest::prelude::*;
use shiplog_ports::Renderer;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::event::EventKind;
use shiplog_testkit::proptest::{
    strategy_coverage_manifest, strategy_event_envelope, strategy_event_vec,
    strategy_workstreams_file,
};

/// Render a packet with the given inputs, returning the output string.
fn render(
    events: &[shiplog_schema::event::EventEnvelope],
    workstreams: &shiplog_schema::workstream::WorkstreamsFile,
    coverage: &shiplog_schema::coverage::CoverageManifest,
) -> String {
    MarkdownRenderer::new()
        .render_packet_markdown("proptest-user", "2024-Q1", events, workstreams, coverage)
        .expect("render_packet_markdown should not fail")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Rendering never panics for arbitrary events, workstreams, and coverage.
    #[test]
    fn rendering_never_panics(
        events in strategy_event_vec(20),
        workstreams in strategy_workstreams_file(),
        coverage in strategy_coverage_manifest(),
    ) {
        let renderer = MarkdownRenderer::new();
        let result = renderer.render_packet_markdown(
            "proptest-user",
            "2024-Q1",
            &events,
            &workstreams,
            &coverage,
        );
        prop_assert!(result.is_ok(), "render_packet_markdown returned Err: {:?}", result.err());
    }

    /// Rendered output always contains the required structural sections.
    #[test]
    fn output_always_contains_required_sections(
        events in strategy_event_vec(10),
        workstreams in strategy_workstreams_file(),
        coverage in strategy_coverage_manifest(),
    ) {
        let out = render(&events, &workstreams, &coverage);

        prop_assert!(out.contains("# Summary"), "Missing Summary heading");
        prop_assert!(out.contains("## Workstreams"), "Missing Workstreams heading");
        prop_assert!(out.contains("## Receipts"), "Missing Receipts heading");
        prop_assert!(out.contains("## Coverage"), "Missing Coverage heading");
        prop_assert!(out.contains("## Appendix: All Receipts"), "Missing Appendix heading");
        prop_assert!(out.contains("## File Artifacts"), "Missing File Artifacts heading");
    }

    /// The event count breakdown in the summary matches the actual input counts.
    #[test]
    fn event_counts_in_summary_match_input(
        events in strategy_event_vec(15),
        coverage in strategy_coverage_manifest(),
    ) {
        let workstreams = shiplog_schema::workstream::WorkstreamsFile {
            version: 1,
            generated_at: chrono::Utc::now(),
            workstreams: vec![],
        };
        let out = render(&events, &workstreams, &coverage);

        let pr_count = events.iter().filter(|e| matches!(e.kind, EventKind::PullRequest)).count();
        let review_count = events.iter().filter(|e| matches!(e.kind, EventKind::Review)).count();
        let manual_count = events.iter().filter(|e| matches!(e.kind, EventKind::Manual)).count();

        let expected = format!("**Events:** {} PRs, {} reviews, {} manual", pr_count, review_count, manual_count);
        prop_assert!(out.contains(&expected), "Expected '{}' in output", expected);
    }

    /// The workstream count in the summary matches the input workstream count.
    #[test]
    fn workstream_count_in_summary_matches_input(
        workstreams in strategy_workstreams_file(),
        coverage in strategy_coverage_manifest(),
    ) {
        let out = render(&[], &workstreams, &coverage);
        let expected = format!("**Workstreams:** {}", workstreams.workstreams.len());
        prop_assert!(out.contains(&expected), "Expected '{}' in output", expected);
    }

    /// CoverageFirst section order puts Coverage before Summary.
    #[test]
    fn coverage_first_order_puts_coverage_before_summary(
        events in strategy_event_vec(5),
        workstreams in strategy_workstreams_file(),
        coverage in strategy_coverage_manifest(),
    ) {
        let renderer = MarkdownRenderer::new()
            .with_section_order(shiplog_render_md::SectionOrder::CoverageFirst);
        let out = renderer
            .render_packet_markdown("user", "2024", &events, &workstreams, &coverage)
            .unwrap();

        let cov_pos = out.find("## Coverage");
        let sum_pos = out.find("# Summary");
        prop_assert!(cov_pos.is_some() && sum_pos.is_some());
        prop_assert!(cov_pos.unwrap() < sum_pos.unwrap(),
            "Coverage should appear before Summary in CoverageFirst order");
    }

    /// File Artifacts section always lists the expected artifact files.
    #[test]
    fn file_artifacts_always_present(
        events in strategy_event_vec(5),
        coverage in strategy_coverage_manifest(),
    ) {
        let workstreams = shiplog_schema::workstream::WorkstreamsFile {
            version: 1,
            generated_at: chrono::Utc::now(),
            workstreams: vec![],
        };
        let out = render(&events, &workstreams, &coverage);

        prop_assert!(out.contains("ledger.events.jsonl"));
        prop_assert!(out.contains("coverage.manifest.json"));
        prop_assert!(out.contains("workstreams.yaml"));
        prop_assert!(out.contains("manual_events.yaml"));
    }

    /// Rendering a single arbitrary event never panics and produces non-empty output.
    #[test]
    fn single_event_produces_nonempty_output(
        event in strategy_event_envelope(),
        coverage in strategy_coverage_manifest(),
    ) {
        let workstreams = shiplog_schema::workstream::WorkstreamsFile {
            version: 1,
            generated_at: chrono::Utc::now(),
            workstreams: vec![],
        };
        let out = render(&[event], &workstreams, &coverage);
        prop_assert!(!out.is_empty());
    }
}
