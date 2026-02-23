//! Profile-string projection dispatch for shiplog redaction.
//!
//! This crate has a narrow responsibility:
//! - parse incoming profile strings into `RedactionProfile`
//! - apply extracted policy transforms with an alias resolver

pub use shiplog_redaction_policy::{AliasResolver, RedactionProfile};
use shiplog_redaction_policy::{redact_events_with_aliases, redact_workstreams_with_aliases};
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::WorkstreamsFile;

/// Parse a raw profile string into a canonical profile.
#[must_use]
pub fn parse_profile(profile: &str) -> RedactionProfile {
    RedactionProfile::from_profile_str(profile)
}

/// Project events using profile-string dispatch and an alias resolver.
#[must_use]
pub fn project_events_with_aliases<A: AliasResolver + ?Sized>(
    events: &[EventEnvelope],
    profile: &str,
    aliases: &A,
) -> Vec<EventEnvelope> {
    redact_events_with_aliases(events, parse_profile(profile), aliases)
}

/// Project workstreams using profile-string dispatch and an alias resolver.
#[must_use]
pub fn project_workstreams_with_aliases<A: AliasResolver + ?Sized>(
    workstreams: &WorkstreamsFile,
    profile: &str,
    aliases: &A,
) -> WorkstreamsFile {
    redact_workstreams_with_aliases(workstreams, parse_profile(profile), aliases)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_profile_keeps_known_values() {
        assert_eq!(parse_profile("internal"), RedactionProfile::Internal);
        assert_eq!(parse_profile("manager"), RedactionProfile::Manager);
        assert_eq!(parse_profile("public"), RedactionProfile::Public);
    }

    #[test]
    fn parse_profile_defaults_unknown_to_public() {
        assert_eq!(parse_profile("unexpected"), RedactionProfile::Public);
        assert_eq!(parse_profile(""), RedactionProfile::Public);
    }
}
