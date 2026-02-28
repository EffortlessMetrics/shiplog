use shiplog_hooks::{
    ClosureHook, Hook, HookContext, HookMetadata, HookRegistry, HookResult, PipelineStage,
};

fn make_context(stage: PipelineStage) -> HookContext {
    HookContext {
        pipeline_stage: stage,
        data: serde_json::json!({"key": "value"}),
        metadata: HookMetadata::new("test"),
    }
}

// --- HookMetadata tests ---

#[test]
fn metadata_source_preserved() {
    let m = HookMetadata::new("my-source");
    assert_eq!(m.source, "my-source");
    assert!(m.correlation_id.is_none());
}

#[test]
fn metadata_correlation_id_chaining() {
    let m = HookMetadata::new("src").with_correlation_id("corr-1");
    assert_eq!(m.correlation_id, Some("corr-1".to_string()));
}

// --- HookResult tests ---

#[test]
fn hook_result_success() {
    let r = HookResult::success();
    assert!(r.success);
    assert!(!r.modified);
    assert!(r.message.is_none());
    assert!(r.data.is_none());
}

#[test]
fn hook_result_success_with_data() {
    let data = serde_json::json!({"count": 5});
    let r = HookResult::success_with_data(data.clone());
    assert!(r.success);
    assert!(r.modified);
    assert_eq!(r.data, Some(data));
}

#[test]
fn hook_result_failure() {
    let r = HookResult::failure("bad thing");
    assert!(!r.success);
    assert_eq!(r.message, Some("bad thing".to_string()));
}

#[test]
fn hook_result_skipped() {
    let r = HookResult::skipped("not applicable");
    assert!(r.success);
    assert!(!r.modified);
    assert_eq!(r.message, Some("not applicable".to_string()));
}

// --- PipelineStage display tests ---

#[test]
fn all_pipeline_stages_display() {
    let expected = vec![
        (PipelineStage::PreIngest, "pre-ingest"),
        (PipelineStage::PostIngest, "post-ingest"),
        (PipelineStage::PreTransform, "pre-transform"),
        (PipelineStage::PostTransform, "post-transform"),
        (PipelineStage::PreRender, "pre-render"),
        (PipelineStage::PostRender, "post-render"),
        (PipelineStage::PreBundle, "pre-bundle"),
        (PipelineStage::PostBundle, "post-bundle"),
        (PipelineStage::PreExport, "pre-export"),
        (PipelineStage::PostExport, "post-export"),
    ];
    for (stage, label) in expected {
        assert_eq!(format!("{}", stage), label);
    }
}

// --- ClosureHook tests ---

#[test]
fn closure_hook_name_and_execute() {
    let hook = ClosureHook::new("my-hook", |_ctx| HookResult::success());
    assert_eq!(hook.name(), "my-hook");

    let ctx = make_context(PipelineStage::PreIngest);
    let result = hook.execute(&ctx);
    assert!(result.success);
}

#[test]
fn closure_hook_accesses_context_data() {
    let hook = ClosureHook::new("data-hook", |ctx| {
        if ctx.data.get("key").is_some() {
            HookResult::success_with_data(serde_json::json!({"found": true}))
        } else {
            HookResult::failure("key not found")
        }
    });

    let ctx = make_context(PipelineStage::PreRender);
    let result = hook.execute(&ctx);
    assert!(result.success);
    assert!(result.modified);
}

// --- HookRegistry tests ---

#[test]
fn registry_new_is_empty() {
    let reg = HookRegistry::new();
    assert!(reg.is_empty());
    assert_eq!(reg.len(), 0);
}

#[test]
fn registry_default_is_empty() {
    let reg = HookRegistry::default();
    assert!(reg.is_empty());
}

#[test]
fn registry_register_increments_len() {
    let mut reg = HookRegistry::new();
    reg.register(
        PipelineStage::PreIngest,
        ClosureHook::new("h1", |_| HookResult::success()),
    );
    assert_eq!(reg.len(), 1);

    reg.register(
        PipelineStage::PreIngest,
        ClosureHook::new("h2", |_| HookResult::success()),
    );
    assert_eq!(reg.len(), 2);

    reg.register(
        PipelineStage::PostIngest,
        ClosureHook::new("h3", |_| HookResult::success()),
    );
    assert_eq!(reg.len(), 3);
}

#[test]
fn registry_execute_no_hooks_for_stage_returns_success() {
    let reg = HookRegistry::new();
    let ctx = make_context(PipelineStage::PreIngest);
    let result = reg.execute(PipelineStage::PreIngest, ctx);
    assert!(result.success);
}

#[test]
fn registry_execute_single_hook() {
    let mut reg = HookRegistry::new();
    reg.register(
        PipelineStage::PreIngest,
        ClosureHook::new("h1", |_| {
            HookResult::success_with_data(serde_json::json!({"modified": true}))
        }),
    );

    let ctx = make_context(PipelineStage::PreIngest);
    let result = reg.execute(PipelineStage::PreIngest, ctx);
    assert!(result.success);
    assert!(result.modified);
}

#[test]
fn registry_execute_multiple_hooks_all_success() {
    let mut reg = HookRegistry::new();
    reg.register(
        PipelineStage::PreRender,
        ClosureHook::new("h1", |_| HookResult::success()),
    );
    reg.register(
        PipelineStage::PreRender,
        ClosureHook::new("h2", |_| HookResult::success()),
    );

    let ctx = make_context(PipelineStage::PreRender);
    let result = reg.execute(PipelineStage::PreRender, ctx);
    assert!(result.success);
}

#[test]
fn registry_execute_one_failure_causes_failure() {
    let mut reg = HookRegistry::new();
    reg.register(
        PipelineStage::PreExport,
        ClosureHook::new("ok", |_| HookResult::success()),
    );
    reg.register(
        PipelineStage::PreExport,
        ClosureHook::new("fail", |_| HookResult::failure("broken")),
    );

    let ctx = make_context(PipelineStage::PreExport);
    let result = reg.execute(PipelineStage::PreExport, ctx);
    assert!(!result.success);
}

#[test]
fn registry_hooks_data_chaining() {
    let mut reg = HookRegistry::new();
    reg.register(
        PipelineStage::PreIngest,
        ClosureHook::new("step1", |_| {
            HookResult::success_with_data(serde_json::json!({"step": 1}))
        }),
    );
    reg.register(
        PipelineStage::PreIngest,
        ClosureHook::new("step2", |ctx| {
            let step = ctx.data.get("step").and_then(|v| v.as_i64()).unwrap_or(0);
            HookResult::success_with_data(serde_json::json!({"step": step + 1}))
        }),
    );

    let ctx = make_context(PipelineStage::PreIngest);
    let result = reg.execute(PipelineStage::PreIngest, ctx);
    assert!(result.success);
    assert_eq!(result.data.unwrap().get("step").unwrap().as_i64(), Some(2));
}

#[test]
fn registry_different_stages_are_independent() {
    let mut reg = HookRegistry::new();
    reg.register(
        PipelineStage::PreIngest,
        ClosureHook::new("a", |_| HookResult::success()),
    );
    reg.register(
        PipelineStage::PostExport,
        ClosureHook::new("b", |_| HookResult::failure("fail")),
    );

    let ctx1 = make_context(PipelineStage::PreIngest);
    assert!(reg.execute(PipelineStage::PreIngest, ctx1).success);

    let ctx2 = make_context(PipelineStage::PostExport);
    assert!(!reg.execute(PipelineStage::PostExport, ctx2).success);
}

// --- Property tests ---

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn metadata_source_roundtrip(source in "[a-zA-Z0-9\\-]{1,30}") {
            let m = HookMetadata::new(&source);
            prop_assert_eq!(&m.source, &source);
        }

        #[test]
        fn registry_n_hooks_has_correct_len(n in 0usize..20) {
            let mut reg = HookRegistry::new();
            for i in 0..n {
                reg.register(
                    PipelineStage::PreIngest,
                    ClosureHook::new(format!("h{}", i), |_| HookResult::success()),
                );
            }
            prop_assert_eq!(reg.len(), n);
        }
    }
}
