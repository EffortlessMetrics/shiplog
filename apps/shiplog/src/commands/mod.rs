//! CLI command dispatch and command-family handlers.
//!
//! Keep `main` focused on process startup while this module routes parsed
//! subcommands to narrow handler modules for the larger pipeline families.

mod collect;
mod import;
mod merge;
mod refresh;
mod run;

use clap::Parser;

use crate::*;

pub(super) fn dispatch() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::Init {
            sources,
            dry_run,
            force,
            guided,
        } => {
            run_init(sources, dry_run, force, guided)?;
        }

        Command::Doctor {
            config,
            sources,
            setup,
            repair_plan,
            json,
        } => {
            if setup {
                run_doctor_setup(&config, &sources, json)?;
            } else if repair_plan {
                run_doctor_repair_plan(&config, &sources)?;
            } else {
                run_doctor(&config, &sources)?;
            }
        }

        Command::Intake(args) => {
            run_intake(args)?;
        }

        Command::Config { cmd } => match cmd {
            ConfigCommand::Validate { config } => {
                run_config_validate(&config)?;
            }
            ConfigCommand::Explain { config } => {
                run_config_explain(&config)?;
            }
            ConfigCommand::Migrate { config, dry_run } => {
                run_config_migrate(&config, dry_run)?;
            }
        },

        Command::Sources { cmd } => match cmd {
            SourcesCommand::Status(args) => run_sources_status(&args.config, &args.sources)?,
        },

        Command::Status(args) => {
            run_status(args)?;
        }

        Command::Github { cmd } => match cmd {
            GithubCommand::Activity { cmd } => match cmd {
                GithubActivityCommand::Plan(args) => github_activity::run_plan(args)?,
                GithubActivityCommand::Scout(args) => github_activity::run_scout(args)?,
                GithubActivityCommand::Run(args) => github_activity::run_activity(args)?,
                GithubActivityCommand::Status(args) => github_activity::run_status(args)?,
                GithubActivityCommand::Report(args) => github_activity::run_report(args)?,
                GithubActivityCommand::Merge(args) => github_activity::run_merge(args)?,
            },
        },

        Command::Periods { cmd } => match cmd {
            PeriodsCommand::List(args) => run_periods_list(args)?,
            PeriodsCommand::Explain(args) => run_periods_explain(args)?,
        },

        Command::Cache { cmd } => match cmd {
            CacheCommand::Stats(args) => run_cache_stats(args)?,
            CacheCommand::Inspect(args) => run_cache_inspect(args)?,
            CacheCommand::Clean(args) => run_cache_clean(args)?,
        },

        Command::Identify { cmd } => match cmd {
            IdentifyCommand::Jira {
                instance,
                auth_user,
                token,
            } => run_identify_jira(instance, auth_user, token)?,
            IdentifyCommand::Linear { api_key } => run_identify_linear(api_key)?,
        },

        Command::Journal { cmd } => match cmd {
            JournalCommand::Add(args) => run_journal_add(args)?,
            JournalCommand::List(args) => run_journal_list(args)?,
            JournalCommand::Edit(args) => run_journal_edit(args)?,
        },

        Command::Collect {
            source,
            out,
            zip,
            redact_key,
            bundle_profile,
            regen,
            llm_cluster,
            llm_api_endpoint,
            llm_model,
            llm_api_key,
        } => collect::handle(
            source,
            out,
            zip,
            redact_key,
            bundle_profile,
            regen,
            llm_cluster,
            llm_api_endpoint,
            llm_model,
            llm_api_key,
        )?,

        Command::Render {
            out,
            run,
            latest,
            user,
            window_label,
            redact_key,
            bundle_profile,
            mode,
            receipt_limit,
            appendix,
            zip,
        } => {
            let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
            let outputs = render_existing_run(RenderExistingArgs {
                out: &out,
                run,
                latest,
                user: Some(&user),
                window_label: Some(&window_label),
                redaction_key,
                bundle_profile: bundle_profile.clone(),
                mode,
                receipt_limit,
                appendix,
                zip,
            })?;

            println!("Rendered from existing events:");
            print_outputs(&outputs, WorkstreamSource::Curated);
        }

        Command::Share { cmd } => match cmd {
            ShareCommand::Manager(options) => {
                let bundle_profile = BundleProfile::Manager;
                let redaction_key =
                    RedactionKey::resolve_for_share(options.redact_key, &bundle_profile)?;
                let outputs = render_existing_run(RenderExistingArgs {
                    out: &options.out,
                    run: options.run,
                    latest: options.latest,
                    user: None,
                    window_label: None,
                    redaction_key: redaction_key.clone(),
                    bundle_profile,
                    mode: RenderPacketMode::Packet,
                    receipt_limit: None,
                    appendix: None,
                    zip: options.zip,
                })?;
                let manifest_path =
                    write_share_manifest(&outputs, &BundleProfile::Manager, &redaction_key)?;
                print_share_outputs(&outputs, &BundleProfile::Manager, &manifest_path);
            }
            ShareCommand::Public(options) => {
                let bundle_profile = BundleProfile::Public;
                let redaction_key =
                    RedactionKey::resolve_for_share(options.redact_key, &bundle_profile)?;
                let outputs = render_existing_run(RenderExistingArgs {
                    out: &options.out,
                    run: options.run,
                    latest: options.latest,
                    user: None,
                    window_label: None,
                    redaction_key: redaction_key.clone(),
                    bundle_profile,
                    mode: RenderPacketMode::Packet,
                    receipt_limit: None,
                    appendix: None,
                    zip: options.zip,
                })?;
                let manifest_path =
                    write_share_manifest(&outputs, &BundleProfile::Public, &redaction_key)?;
                print_share_outputs(&outputs, &BundleProfile::Public, &manifest_path);
            }
            ShareCommand::Explain { cmd } => match cmd {
                ShareExplainCommand::Manager(options) => {
                    explain_share_profile(options, BundleProfile::Manager)?;
                }
                ShareExplainCommand::Public(options) => {
                    explain_share_profile(options, BundleProfile::Public)?;
                }
            },
            ShareCommand::Verify { cmd } => match cmd {
                ShareVerifyCommand::Manager(options) => {
                    verify_share_profile(options, BundleProfile::Manager)?;
                }
                ShareVerifyCommand::Public(options) => {
                    verify_share_profile(options, BundleProfile::Public)?;
                }
                ShareVerifyCommand::Manifest(options) => {
                    verify_share_manifest(options)?;
                }
            },
        },

        Command::Refresh {
            source,
            out,
            run_dir: explicit_run_dir,
            zip,
            redact_key,
            bundle_profile,
        } => refresh::handle(
            source,
            out,
            explicit_run_dir,
            zip,
            redact_key,
            bundle_profile,
        )?,
        Command::Workstreams { cmd } => match cmd {
            WorkstreamsCommand::List { out, run, latest } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (workstreams, source, path) = load_effective_workstreams_for_run(&run_dir)?;
                print_workstreams_list(&run_dir, &path, source, &workstreams);
            }
            WorkstreamsCommand::Validate { out, run, latest } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (workstreams, source, path) = load_effective_workstreams_for_run(&run_dir)?;
                let errors = validate_workstreams_for_run(&run_dir, &workstreams)?;
                if errors.is_empty() {
                    println!(
                        "Workstreams valid: {} ({})",
                        path.display(),
                        workstream_source_label(source)
                    );
                    println!("- {} workstreams", workstreams.workstreams.len());
                    println!(
                        "- {} assigned events",
                        workstreams
                            .workstreams
                            .iter()
                            .map(|workstream| workstream.events.len())
                            .sum::<usize>()
                    );
                    println!(
                        "- {} receipts",
                        workstreams
                            .workstreams
                            .iter()
                            .map(|workstream| workstream.receipts.len())
                            .sum::<usize>()
                    );
                } else {
                    for error in &errors {
                        eprintln!("- {error}");
                    }
                    anyhow::bail!("{} workstream validation error(s)", errors.len());
                }
            }
            WorkstreamsCommand::Rename {
                out,
                run,
                latest,
                from,
                to,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (mut workstreams, source, _) = load_effective_workstreams_for_run(&run_dir)?;
                let old_title = rename_workstream(&mut workstreams, &from, &to)?;
                write_curated_workstreams(&run_dir, &workstreams)?;
                println!("Renamed workstream: {old_title} -> {}", to.trim());
                println!(
                    "Updated: {}",
                    shiplog::workstreams::WorkstreamManager::curated_path(&run_dir).display()
                );
                if matches!(source, WorkstreamsFileSource::Suggested) {
                    println!("Created curated workstreams.yaml from suggested workstreams.");
                }
            }
            WorkstreamsCommand::Move {
                out,
                run,
                latest,
                event,
                to,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (mut workstreams, source, _) = load_effective_workstreams_for_run(&run_dir)?;
                let ledger_events = load_run_events(&run_dir)?;
                let result =
                    move_event_to_workstream(&mut workstreams, &event, &to, &ledger_events)?;
                let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                if !errors.is_empty() {
                    for error in &errors {
                        eprintln!("- {error}");
                    }
                    anyhow::bail!("{} workstream validation error(s)", errors.len());
                }

                write_curated_workstreams(&run_dir, &workstreams)?;
                println!("Moved event {} to {}", result.event_id, result.to_title);
                if result.from_titles.is_empty() {
                    println!("Source: unassigned");
                } else {
                    println!("Source: {}", result.from_titles.join(", "));
                }
                if result.receipt_preserved {
                    println!("Receipt anchor preserved in target workstream.");
                }
                println!(
                    "Updated: {}",
                    shiplog::workstreams::WorkstreamManager::curated_path(&run_dir).display()
                );
                if matches!(source, WorkstreamsFileSource::Suggested) {
                    println!("Created curated workstreams.yaml from suggested workstreams.");
                }
            }
            WorkstreamsCommand::Receipts {
                out,
                run,
                latest,
                workstream,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (workstreams, source, path) = load_effective_workstreams_for_run(&run_dir)?;
                let ledger_events = load_run_events(&run_dir)?;
                print_workstream_receipts(
                    &run_dir,
                    &path,
                    source,
                    &workstreams,
                    &ledger_events,
                    &workstream,
                )?;
            }
            WorkstreamsCommand::Receipt { cmd } => match cmd {
                WorkstreamReceiptCommand::Add {
                    out,
                    run,
                    latest,
                    workstream,
                    event,
                } => {
                    let run_dir = resolve_render_run_dir(&out, run, latest)?;
                    let (mut workstreams, source, _) =
                        load_effective_workstreams_for_run(&run_dir)?;
                    let ledger_events = load_run_events(&run_dir)?;
                    let result = add_workstream_receipt(
                        &mut workstreams,
                        &workstream,
                        &event,
                        &ledger_events,
                    )?;
                    let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                    if !errors.is_empty() {
                        for error in &errors {
                            eprintln!("- {error}");
                        }
                        anyhow::bail!("{} workstream validation error(s)", errors.len());
                    }

                    write_curated_workstreams(&run_dir, &workstreams)?;
                    println!(
                        "Added receipt anchor {} to {}",
                        result.event_id, result.workstream_title
                    );
                    println!("Receipt: {}", result.event_title);
                    println!(
                        "Updated: {}",
                        shiplog::workstreams::WorkstreamManager::curated_path(&run_dir).display()
                    );
                    if matches!(source, WorkstreamsFileSource::Suggested) {
                        println!("Created curated workstreams.yaml from suggested workstreams.");
                    }
                }
                WorkstreamReceiptCommand::Remove {
                    out,
                    run,
                    latest,
                    workstream,
                    event,
                } => {
                    let run_dir = resolve_render_run_dir(&out, run, latest)?;
                    let (mut workstreams, source, _) =
                        load_effective_workstreams_for_run(&run_dir)?;
                    let ledger_events = load_run_events(&run_dir)?;
                    let result = remove_workstream_receipt(
                        &mut workstreams,
                        &workstream,
                        &event,
                        &ledger_events,
                    )?;
                    let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                    if !errors.is_empty() {
                        for error in &errors {
                            eprintln!("- {error}");
                        }
                        anyhow::bail!("{} workstream validation error(s)", errors.len());
                    }

                    write_curated_workstreams(&run_dir, &workstreams)?;
                    println!(
                        "Removed receipt anchor {} from {}",
                        result.event_id, result.workstream_title
                    );
                    println!("Receipt: {}", result.event_title);
                    println!(
                        "Updated: {}",
                        shiplog::workstreams::WorkstreamManager::curated_path(&run_dir).display()
                    );
                    if matches!(source, WorkstreamsFileSource::Suggested) {
                        println!("Created curated workstreams.yaml from suggested workstreams.");
                    }
                }
            },
            WorkstreamsCommand::Create {
                out,
                run,
                latest,
                title,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (mut workstreams, source, _) = load_effective_workstreams_for_run(&run_dir)?;
                let result = create_workstream(&mut workstreams, &title)?;
                let ledger_events = load_run_events(&run_dir)?;
                let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                if !errors.is_empty() {
                    for error in &errors {
                        eprintln!("- {error}");
                    }
                    anyhow::bail!("{} workstream validation error(s)", errors.len());
                }

                write_curated_workstreams(&run_dir, &workstreams)?;
                println!("Created workstream: {}", result.title);
                println!("ID: {}", result.id);
                println!(
                    "Updated: {}",
                    shiplog::workstreams::WorkstreamManager::curated_path(&run_dir).display()
                );
                if matches!(source, WorkstreamsFileSource::Suggested) {
                    println!("Created curated workstreams.yaml from suggested workstreams.");
                }
            }
            WorkstreamsCommand::Delete {
                out,
                run,
                latest,
                workstream,
                move_to,
                force,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (mut workstreams, source, _) = load_effective_workstreams_for_run(&run_dir)?;
                let ledger_events = load_run_events(&run_dir)?;
                let result = delete_workstream(
                    &mut workstreams,
                    &workstream,
                    move_to.as_deref(),
                    force,
                    &ledger_events,
                )?;
                let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                if !errors.is_empty() {
                    for error in &errors {
                        eprintln!("- {error}");
                    }
                    anyhow::bail!("{} workstream validation error(s)", errors.len());
                }

                write_curated_workstreams(&run_dir, &workstreams)?;
                println!("Deleted workstream: {}", result.deleted_title);
                if let Some(target) = result.moved_to_title {
                    println!(
                        "Moved {} event(s) and {} receipt anchor(s) to {}.",
                        result.event_count, result.receipt_count, target
                    );
                } else if result.event_count > 0 || result.receipt_count > 0 {
                    println!(
                        "Discarded {} event assignment(s) and {} receipt anchor(s).",
                        result.event_count, result.receipt_count
                    );
                }
                println!(
                    "Updated: {}",
                    shiplog::workstreams::WorkstreamManager::curated_path(&run_dir).display()
                );
                if matches!(source, WorkstreamsFileSource::Suggested) {
                    println!("Created curated workstreams.yaml from suggested workstreams.");
                }
            }
            WorkstreamsCommand::Split {
                out,
                run,
                latest,
                from,
                to,
                matching,
                create,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (mut workstreams, source, _) = load_effective_workstreams_for_run(&run_dir)?;
                let ledger_events = load_run_events(&run_dir)?;
                let result = split_workstream(
                    &mut workstreams,
                    &from,
                    &to,
                    &matching,
                    create,
                    &ledger_events,
                )?;
                let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                if !errors.is_empty() {
                    for error in &errors {
                        eprintln!("- {error}");
                    }
                    anyhow::bail!("{} workstream validation error(s)", errors.len());
                }

                write_curated_workstreams(&run_dir, &workstreams)?;
                println!(
                    "Split {} event(s) from {} to {}",
                    result.event_count, result.from_title, result.to_title
                );
                println!("Matched: {}", result.pattern);
                if result.receipt_count > 0 {
                    println!("Moved {} receipt anchor(s).", result.receipt_count);
                }
                if result.created_target {
                    println!("Created target workstream: {}", result.to_title);
                }
                println!(
                    "Updated: {}",
                    shiplog::workstreams::WorkstreamManager::curated_path(&run_dir).display()
                );
                if matches!(source, WorkstreamsFileSource::Suggested) {
                    println!("Created curated workstreams.yaml from suggested workstreams.");
                }
            }
        },
        Command::Runs { cmd } => match cmd {
            RunsCommand::List { out } => {
                let summaries = load_run_summaries(&out)?;
                print_runs_list(&out, &summaries);
            }
            RunsCommand::Show { out, run, latest } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let summary = load_run_summary(&run_dir)?;
                print_run_show(&summary);
            }
            RunsCommand::Compare {
                out,
                config,
                from,
                from_period,
                to,
                to_period,
            } => {
                let from_dir = resolve_compare_run_dir(&out, &config, "from", from, from_period)?;
                let to_dir = resolve_compare_run_dir(&out, &config, "to", to, to_period)?;
                let comparison = compare_runs(&from_dir, &to_dir)?;
                print_run_compare(&comparison, &out);
            }
            RunsCommand::Diff {
                out,
                latest,
                from,
                to,
            } => {
                run_quality_diff_command(&out, latest, from, to)?;
            }
        },
        Command::Review { cmd, options } => match cmd {
            Some(ReviewCommand::Weekly {
                out,
                run,
                latest,
                strict,
            }) => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                print_weekly_review(&run_dir, &out, strict)?;
            }
            Some(ReviewCommand::Fixups {
                out,
                run,
                latest,
                commands_only,
                journal_template,
            }) => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                print_review_fixups(&run_dir, &out, commands_only, journal_template)?;
            }
            None => {
                let run_dir = resolve_review_run_dir(
                    &options.out,
                    options.run,
                    options.latest,
                    &options.config,
                    options.period,
                )?;
                print_review(&run_dir, &options.out, options.strict)?;
            }
        },
        Command::Open { cmd } => match cmd {
            OpenCommand::Packet {
                out,
                run,
                latest,
                print_path,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let packet = run_dir.join("packet.md");
                open_existing_path(
                    &packet,
                    "Packet",
                    "Run `shiplog render --latest` to create it.",
                    print_path,
                )?;
            }
            OpenCommand::Workstreams {
                out,
                run,
                latest,
                print_path,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (_, _, path) = load_effective_workstreams_for_run(&run_dir)?;
                open_existing_path(
                    &path,
                    "Workstreams file",
                    "Run `shiplog collect` first.",
                    print_path,
                )?;
            }
            OpenCommand::Out {
                out,
                run,
                latest,
                print_path,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                open_existing_path(
                    &run_dir,
                    "Run directory",
                    "Run `shiplog collect` first.",
                    print_path,
                )?;
            }
            OpenCommand::IntakeReport {
                out,
                run,
                latest,
                print_path,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let report = run_dir.join("intake.report.md");
                open_existing_path(
                    &report,
                    "Intake report",
                    "Run `shiplog intake` first.",
                    print_path,
                )?;
            }
        },
        Command::Report { cmd } => match cmd {
            ReportCommand::Validate {
                out,
                run,
                latest,
                path,
            } => {
                validate_intake_report_command(&out, run, latest, path)?;
            }
            ReportCommand::Summarize {
                out,
                run,
                latest,
                path,
            } => {
                summarize_intake_report_command(&out, run, latest, path)?;
            }
            ReportCommand::ExportAgentPack {
                out,
                run,
                latest,
                path,
                output,
            } => {
                export_agent_pack_command(&out, run, latest, path, output)?;
            }
        },
        Command::Repair { cmd } => match cmd {
            RepairCommand::Plan { out, run, latest } => {
                repair_plan_command(&out, run, latest)?;
            }
            RepairCommand::Diff { out, latest } => {
                repair_diff_command(&out, latest)?;
            }
        },
        Command::Merge {
            inputs,
            out,
            conflict,
            user,
            window_label,
            zip,
            redact_key,
            bundle_profile,
            regen,
        } => merge::handle(
            inputs,
            out,
            conflict,
            user,
            window_label,
            zip,
            redact_key,
            bundle_profile,
            regen,
        )?,
        Command::Import {
            dir,
            out,
            user,
            window_label,
            redact_key,
            bundle_profile,
            zip,
            regen,
            llm_cluster,
            llm_api_endpoint,
            llm_model,
            llm_api_key,
        } => import::handle(
            dir,
            out,
            user,
            window_label,
            redact_key,
            bundle_profile,
            zip,
            regen,
            llm_cluster,
            llm_api_endpoint,
            llm_model,
            llm_api_key,
        )?,

        Command::Run {
            source,
            out,
            zip,
            redact_key,
            bundle_profile,
            llm_cluster,
            llm_api_endpoint,
            llm_model,
            llm_api_key,
        } => run::handle(
            source,
            out,
            zip,
            redact_key,
            bundle_profile,
            llm_cluster,
            llm_api_endpoint,
            llm_model,
            llm_api_key,
        )?,
    }

    Ok(())
}
