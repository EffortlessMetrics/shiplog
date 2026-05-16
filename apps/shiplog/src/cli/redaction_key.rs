use anyhow::Result;
use shiplog::schema::bundle::BundleProfile;

#[derive(Debug, Clone)]
pub(crate) struct RedactionKey {
    key: Option<String>,
    source: RedactionKeySource,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum RedactionKeySource {
    Explicit,
    Env,
    None,
}

impl RedactionKeySource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Env => "env",
            Self::None => "none",
        }
    }
}

impl RedactionKey {
    pub(crate) fn resolve(
        redact_key: Option<String>,
        bundle_profile: &BundleProfile,
    ) -> Result<Self> {
        Self::resolve_with_env(redact_key, bundle_profile, "SHIPLOG_REDACT_KEY")
    }

    pub(crate) fn resolve_for_share(
        redact_key: Option<String>,
        bundle_profile: &BundleProfile,
    ) -> Result<Self> {
        let key_env = "SHIPLOG_REDACT_KEY";
        let (key, source) = resolve_redaction_key(redact_key, key_env);
        if key.is_none() {
            core::hint::cold_path();
            anyhow::bail!(share_command_key_error(bundle_profile, key_env));
        }
        Ok(Self { key, source })
    }

    pub(crate) fn resolve_with_env(
        redact_key: Option<String>,
        bundle_profile: &BundleProfile,
        key_env: &str,
    ) -> Result<Self> {
        let (key, source) = resolve_redaction_key(redact_key, key_env);
        if key.is_none() && !matches!(bundle_profile, BundleProfile::Internal) {
            core::hint::cold_path();
            anyhow::bail!(share_profile_key_error(bundle_profile, key_env));
        }
        Ok(Self { key, source })
    }

    pub(crate) fn engine_key(&self) -> &str {
        self.key.as_deref().unwrap_or("")
    }

    pub(crate) fn render_profiles(&self) -> bool {
        self.key.is_some()
    }

    pub(crate) fn source(&self) -> RedactionKeySource {
        self.source
    }
}

pub(crate) fn resolve_redaction_key(
    redact_key: Option<String>,
    key_env: &str,
) -> (Option<String>, RedactionKeySource) {
    if let Some(key) = redact_key {
        return (Some(key), RedactionKeySource::Explicit);
    }
    if let Ok(key) = std::env::var(key_env) {
        return (Some(key), RedactionKeySource::Env);
    }
    (None, RedactionKeySource::None)
}

fn share_profile_key_error(bundle_profile: &BundleProfile, key_env: &str) -> String {
    format!(
        "{bundle_profile} profile requires --redact-key or {key_env}.\n\
         Try:\n\
           export {key_env}=replace-with-a-stable-secret\n\
           rerun this command with --bundle-profile {bundle_profile}\n\
         For an internal-only packet, use --bundle-profile internal."
    )
}

fn share_command_key_error(bundle_profile: &BundleProfile, key_env: &str) -> String {
    format!(
        "{bundle_profile} share requires --redact-key or {key_env}.\n\
         Try:\n\
           export {key_env}=replace-with-a-stable-secret\n\
           shiplog share {bundle_profile} --latest\n\
         For an internal-only packet, use `shiplog render --bundle-profile internal`."
    )
}
