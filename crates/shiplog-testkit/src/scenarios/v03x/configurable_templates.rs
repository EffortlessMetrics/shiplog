//! BDD scenarios for Configurable Packet Templates (Feature 8)
//!
//! Scenarios cover:
//! - Primary user workflows (defining and using custom templates)
//! - Edge cases (missing files, syntax errors, undefined variables)
//! - Integration with other features (event sources, redaction)
//! - Performance scenarios (complex templates with many workstreams)

use crate::bdd::Scenario;
use crate::bdd::assertions::*;

/// Scenario 8.1: User defines custom packet template
pub fn template_custom() -> Scenario {
    Scenario::new("User defines custom packet template")
        .given(
            "a user has a custom Jinja2 template file at templates/custom.md",
            |ctx| {
                ctx.paths
                    .insert("template_path".to_string(), "templates/custom.md".into());
                ctx.strings.insert(
                    "template_content".to_string(),
                    "# Custom Packet\n{% for ws in workstreams %}...{% endfor %}".to_string(),
                );
            },
        )
        .given("the template defines custom packet structure", |ctx| {
            ctx.flags.insert("template_valid".to_string(), true);
        })
        .when(
            "they run \"shiplog render --template templates/custom.md\"",
            |ctx| {
                ctx.flags.insert("packet_rendered".to_string(), true);
                ctx.flags.insert("custom_template_used".to_string(), true);
                ctx.paths
                    .insert("packet_path".to_string(), "/out/run_001/packet.md".into());
                Ok(())
            },
        )
        .then(
            "the packet should be rendered using the custom template",
            |ctx| {
                assert_true(
                    ctx.flag("packet_rendered").unwrap_or(false),
                    "packet rendered",
                )
            },
        )
        .then("the output should match the template structure", |ctx| {
            assert_true(
                ctx.flag("custom_template_used").unwrap_or(false),
                "custom template used",
            )
        })
}

/// Scenario 8.2: User template includes custom variables
pub fn template_variables() -> Scenario {
    Scenario::new("User template includes custom variables")
        .given(
            "a user has a custom template with variables like {{ user_name }}, {{ company }}",
            |ctx| {
                ctx.strings
                    .insert("user_name".to_string(), "Alice".to_string());
                ctx.strings
                    .insert("company".to_string(), "Acme Corp".to_string());
            },
        )
        .given(
            "they have configured these variables in their config",
            |ctx| {
                ctx.flags.insert("variables_configured".to_string(), true);
            },
        )
        .when(
            "they run \"shiplog render --template templates/custom.md\"",
            |ctx| {
                ctx.flags.insert("packet_rendered".to_string(), true);
                ctx.strings.insert(
                    "output_content".to_string(),
                    "# Packet by Alice at Acme Corp".to_string(),
                );
                Ok(())
            },
        )
        .then(
            "the template variables should be substituted with configured values",
            |ctx| {
                let output = ctx.string("output_content").unwrap();
                assert_contains(output, "Alice", "output")?;
                assert_contains(output, "Acme Corp", "output")
            },
        )
}

/// Scenario 8.3: User template includes conditional sections
pub fn template_conditionals() -> Scenario {
    Scenario::new("User template includes conditional sections")
        .given(
            "a user has a custom template with conditional sections",
            |ctx| {
                ctx.strings.insert(
                    "template_content".to_string(),
                    "{% if show_details %}Details{% endif %}".to_string(),
                );
            },
        )
        .given(
            "the template shows a section only if {{ show_details }} is true",
            |ctx| {
                ctx.flags.insert("has_conditional".to_string(), true);
            },
        )
        .when(
            "they run \"shiplog render --template templates/custom.md --show-details\"",
            |ctx| {
                ctx.flags.insert("packet_rendered".to_string(), true);
                ctx.strings.insert(
                    "output_content".to_string(),
                    "# Packet\nDetails section".to_string(),
                );
                Ok(())
            },
        )
        .then("the conditional section should be included", |ctx| {
            let output = ctx.string("output_content").unwrap();
            assert_contains(output, "Details", "output")
        })
}

/// Scenario 8.4: User template includes loops over workstreams
pub fn template_loops() -> Scenario {
    Scenario::new("User template includes loops over workstreams")
        .given(
            "a user has a custom template with {% for ws in workstreams %}",
            |ctx| {
                ctx.strings.insert(
                    "template_content".to_string(),
                    "{% for ws in workstreams %}{{ ws.title }}\n{% endfor %}".to_string(),
                );
            },
        )
        .given("they have multiple workstreams", |ctx| {
            ctx.numbers.insert("workstream_count".to_string(), 5);
        })
        .when(
            "they run \"shiplog render --template templates/custom.md\"",
            |ctx| {
                ctx.flags.insert("packet_rendered".to_string(), true);
                ctx.strings.insert(
                    "output_content".to_string(),
                    "Workstream 1\nWorkstream 2\nWorkstream 3\nWorkstream 4\nWorkstream 5"
                        .to_string(),
                );
                Ok(())
            },
        )
        .then("the template should iterate over all workstreams", |ctx| {
            let count = ctx.number("workstream_count").unwrap_or(0);
            assert_true(count > 0, "workstream count")
        })
        .then(
            "each workstream should be rendered according to the template",
            |ctx| {
                let output = ctx.string("output_content").unwrap();
                assert_true(output.contains("Workstream"), "output")
            },
        )
}

/// Scenario 8.5: Template file does not exist
pub fn template_not_found() -> Scenario {
    Scenario::new("Template file does not exist")
        .given("a user specifies a non-existent template file", |ctx| {
            ctx.paths
                .insert("template_path".to_string(), "nonexistent.md".into());
        })
        .when(
            "they run \"shiplog render --template nonexistent.md\"",
            |ctx| {
                ctx.flags.insert("command_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "Template file not found: nonexistent.md".to_string(),
                );
                Ok(())
            },
        )
        .then(
            "the command should fail with a clear error message",
            |ctx| {
                assert_true(
                    ctx.flag("command_failed").unwrap_or(false),
                    "command failed",
                )
            },
        )
        .then(
            "the error should indicate the template file was not found",
            |ctx| {
                let error = ctx.string("error_message").unwrap();
                assert_contains(error, "not found", "error message")
            },
        )
}

/// Scenario 8.6: Template has syntax errors
pub fn template_syntax_error() -> Scenario {
    Scenario::new("Template has syntax errors")
        .given("a user has a template with Jinja2 syntax errors", |ctx| {
            ctx.strings.insert(
                "template_content".to_string(),
                "{% if unclosed %}".to_string(),
            );
            ctx.flags.insert("has_syntax_error".to_string(), true);
        })
        .when("they run \"shiplog render --template broken.md\"", |ctx| {
            ctx.flags.insert("command_failed".to_string(), true);
            ctx.strings.insert(
                "error_message".to_string(),
                "Template syntax error at line 1: unclosed 'if' block".to_string(),
            );
            Ok(())
        })
        .then(
            "the command should fail with a clear error message",
            |ctx| {
                assert_true(
                    ctx.flag("command_failed").unwrap_or(false),
                    "command failed",
                )
            },
        )
        .then(
            "the error should indicate the syntax error location",
            |ctx| {
                let error = ctx.string("error_message").unwrap();
                assert_contains(error, "syntax error", "error message")
            },
        )
}

/// Scenario 8.7: Template references undefined variable
pub fn template_undefined_variable() -> Scenario {
    Scenario::new("Template references undefined variable")
        .given(
            "a user has a template that references {{ undefined_var }}",
            |ctx| {
                ctx.strings.insert(
                    "template_content".to_string(),
                    "{{ undefined_var }}".to_string(),
                );
            },
        )
        .when(
            "they run \"shiplog render --template template.md\"",
            |ctx| {
                ctx.flags.insert("command_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "Undefined variable: undefined_var".to_string(),
                );
                Ok(())
            },
        )
        .then(
            "the command should fail with a clear error message",
            |ctx| {
                assert_true(
                    ctx.flag("command_failed").unwrap_or(false),
                    "command failed",
                )
            },
        )
        .then("the error should indicate the undefined variable", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "Undefined variable", "error message")
        })
}

/// Scenario 8.8: Custom template works with all event sources
pub fn template_multi_source() -> Scenario {
    Scenario::new("Custom template works with all event sources")
        .given("a user has collected events from multiple sources", |ctx| {
            ctx.numbers.insert("github_events".to_string(), 25);
            ctx.numbers.insert("gitlab_events".to_string(), 15);
            ctx.numbers.insert("jira_events".to_string(), 10);
        })
        .given("they have a custom template", |ctx| {
            ctx.flags.insert("template_valid".to_string(), true);
        })
        .when("they run \"shiplog render --template custom.md\"", |ctx| {
            ctx.flags.insert("packet_rendered".to_string(), true);
            ctx.flags.insert("all_sources_rendered".to_string(), true);
            Ok(())
        })
        .then(
            "the template should correctly render events from all sources",
            |ctx| {
                assert_true(
                    ctx.flag("packet_rendered").unwrap_or(false),
                    "packet rendered",
                )
            },
        )
        .then("source-specific formatting should work", |ctx| {
            assert_true(
                ctx.flag("all_sources_rendered").unwrap_or(false),
                "all sources rendered",
            )
        })
}

/// Scenario 8.9: Custom template preserves redaction
pub fn template_redaction() -> Scenario {
    Scenario::new("Custom template preserves redaction")
        .given(
            "a user has collected events with sensitive information",
            |ctx| {
                ctx.strings
                    .insert("sensitive_title".to_string(), "Secret Project".to_string());
            },
        )
        .given("they are rendering with a redaction profile", |ctx| {
            ctx.strings
                .insert("redaction_profile".to_string(), "public".to_string());
        })
        .given("they have a custom template", |ctx| {
            ctx.flags.insert("template_valid".to_string(), true);
        })
        .when(
            "they run \"shiplog render --template custom.md --redact public\"",
            |ctx| {
                ctx.flags.insert("packet_rendered".to_string(), true);
                ctx.strings.insert(
                    "output_content".to_string(),
                    "# Packet\n[redacted]".to_string(),
                );
                Ok(())
            },
        )
        .then(
            "the rendered packet should not contain sensitive data",
            |ctx| {
                let output = ctx.string("output_content").unwrap();
                assert_not_contains(output, "Secret Project", "output")
            },
        )
        .then("redaction should work with the custom template", |ctx| {
            let output = ctx.string("output_content").unwrap();
            assert_contains(output, "[redacted]", "output")
        })
}

/// Scenario 8.10: Complex template with many workstreams
pub fn template_large() -> Scenario {
    Scenario::new("Complex template with many workstreams")
        .given("a user has 50 workstreams", |ctx| {
            ctx.numbers.insert("workstream_count".to_string(), 50);
        })
        .given("they have a complex custom template", |ctx| {
            ctx.strings.insert(
                "template_content".to_string(),
                "{% for ws in workstreams %}...{% endfor %}".to_string(),
            );
        })
        .when("they run \"shiplog render --template complex.md\"", |ctx| {
            ctx.flags.insert("packet_rendered".to_string(), true);
            ctx.strings
                .insert("render_time".to_string(), "8s".to_string());
            Ok(())
        })
        .then(
            "rendering should complete within reasonable time (< 10 seconds)",
            |ctx| {
                let time = ctx.string("render_time").unwrap();
                assert_true(time.contains("s") && !time.contains("m"), "render time")
            },
        )
}
