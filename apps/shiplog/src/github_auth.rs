use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const GH_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubAuthSource {
    GhToken,
    GithubToken,
    GhEnterpriseToken,
    GithubEnterpriseToken,
    GhCli,
    Unavailable,
}

impl GithubAuthSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::GhToken => "GH_TOKEN",
            Self::GithubToken => "GITHUB_TOKEN",
            Self::GhEnterpriseToken => "GH_ENTERPRISE_TOKEN",
            Self::GithubEnterpriseToken => "GITHUB_ENTERPRISE_TOKEN",
            Self::GhCli => "gh_cli",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubAuthAvailability {
    Ready,
    Unavailable,
}

impl GithubAuthAvailability {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubAuthReason {
    InvalidApiBase,
    MissingCredential,
    GhUnavailable,
    GhLoggedOut,
    GhMalformedOutput,
    GhCommandFailed,
    GhTimedOut,
    GhHostAmbiguous,
}

impl GithubAuthReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::InvalidApiBase => "invalid_api_base",
            Self::MissingCredential => "missing_credential",
            Self::GhUnavailable => "gh_unavailable",
            Self::GhLoggedOut => "gh_logged_out",
            Self::GhMalformedOutput => "gh_malformed_output",
            Self::GhCommandFailed => "gh_command_failed",
            Self::GhTimedOut => "gh_timed_out",
            Self::GhHostAmbiguous => "gh_host_ambiguous",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GithubAuthMetadata {
    pub source: GithubAuthSource,
    pub host: String,
    pub account: Option<String>,
    pub availability: GithubAuthAvailability,
    pub reason: Option<GithubAuthReason>,
}

pub struct GithubCredential {
    secret: String,
    metadata: GithubAuthMetadata,
}

impl GithubCredential {
    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn metadata(&self) -> &GithubAuthMetadata {
        &self.metadata
    }
}

pub enum GithubAuthResolution {
    Available(GithubCredential),
    Unavailable(GithubAuthMetadata),
}

impl GithubAuthResolution {
    pub fn metadata(&self) -> &GithubAuthMetadata {
        match self {
            Self::Available(credential) => credential.metadata(),
            Self::Unavailable(metadata) => metadata,
        }
    }
}

pub fn resolve(api_base: Option<&str>) -> GithubAuthResolution {
    let host = match github_host(api_base) {
        Ok(host) => host,
        Err(reason) => {
            return GithubAuthResolution::Unavailable(GithubAuthMetadata {
                source: GithubAuthSource::Unavailable,
                host: String::new(),
                account: None,
                availability: GithubAuthAvailability::Unavailable,
                reason: Some(reason),
            });
        }
    };

    let environment = std::env::vars().collect::<BTreeMap<_, _>>();
    if let Some((source, secret)) = environment_credential(&host, &environment) {
        return GithubAuthResolution::Available(GithubCredential {
            secret,
            metadata: GithubAuthMetadata {
                source,
                host,
                account: None,
                availability: GithubAuthAvailability::Ready,
                reason: None,
            },
        });
    }

    resolve_from_gh(host)
}

fn github_host(api_base: Option<&str>) -> Result<String, GithubAuthReason> {
    let api_base = api_base.unwrap_or("https://api.github.com");
    let without_scheme = api_base
        .strip_prefix("https://")
        .or_else(|| api_base.strip_prefix("http://"))
        .ok_or(GithubAuthReason::InvalidApiBase)?;
    let host = without_scheme
        .split('/')
        .next()
        .filter(|value| !value.is_empty())
        .ok_or(GithubAuthReason::InvalidApiBase)?;
    if host.contains(':') || host.chars().any(char::is_whitespace) {
        return Err(GithubAuthReason::InvalidApiBase);
    }
    let host = host.to_ascii_lowercase();
    Ok(if host == "api.github.com" {
        "github.com".to_owned()
    } else {
        host
    })
}

fn environment_credential(
    host: &str,
    environment: &BTreeMap<String, String>,
) -> Option<(GithubAuthSource, String)> {
    let candidates = if host == "github.com" {
        [
            ("GH_TOKEN", GithubAuthSource::GhToken),
            ("GITHUB_TOKEN", GithubAuthSource::GithubToken),
        ]
        .as_slice()
    } else {
        [
            ("GH_ENTERPRISE_TOKEN", GithubAuthSource::GhEnterpriseToken),
            (
                "GITHUB_ENTERPRISE_TOKEN",
                GithubAuthSource::GithubEnterpriseToken,
            ),
        ]
        .as_slice()
    };

    candidates.iter().find_map(|(name, source)| {
        let value = environment.get(*name)?.trim();
        (!value.is_empty()).then(|| (*source, value.to_owned()))
    })
}

#[derive(Debug, Deserialize)]
struct GhAuthStatus {
    hosts: BTreeMap<String, Vec<GhHost>>,
}

#[derive(Debug, Deserialize)]
struct GhHost {
    user: Option<String>,
}

fn resolve_from_gh(host: String) -> GithubAuthResolution {
    let metadata = |account: Option<String>, reason: Option<GithubAuthReason>| GithubAuthMetadata {
        source: if reason.is_none() {
            GithubAuthSource::GhCli
        } else {
            GithubAuthSource::Unavailable
        },
        host: host.clone(),
        account,
        availability: if reason.is_none() {
            GithubAuthAvailability::Ready
        } else {
            GithubAuthAvailability::Unavailable
        },
        reason,
    };

    let status_output = match run_gh(&["auth", "status", "--json", "hosts"]) {
        Ok(output) if output.status.success() => output,
        Ok(_) => {
            return GithubAuthResolution::Unavailable(metadata(
                None,
                Some(GithubAuthReason::GhLoggedOut),
            ));
        }
        Err(reason) => return GithubAuthResolution::Unavailable(metadata(None, Some(reason))),
    };

    let status = match serde_json::from_slice::<GhAuthStatus>(&status_output.stdout) {
        Ok(status) => status,
        Err(_) => {
            return GithubAuthResolution::Unavailable(metadata(
                None,
                Some(GithubAuthReason::GhMalformedOutput),
            ));
        }
    };
    if status.hosts.is_empty() {
        return GithubAuthResolution::Unavailable(metadata(
            None,
            Some(GithubAuthReason::GhLoggedOut),
        ));
    }
    let account = status
        .hosts
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(&host))
        .and_then(|(_, entries)| entries.first())
        .and_then(|entry| entry.user.clone());
    if !status
        .hosts
        .keys()
        .any(|candidate| candidate.eq_ignore_ascii_case(&host))
    {
        return GithubAuthResolution::Unavailable(metadata(
            None,
            Some(GithubAuthReason::GhHostAmbiguous),
        ));
    }

    let token_output = match run_gh(&["auth", "token", "--hostname", &host]) {
        Ok(output) if output.status.success() => output,
        Ok(_) => {
            return GithubAuthResolution::Unavailable(metadata(
                account,
                Some(GithubAuthReason::GhCommandFailed),
            ));
        }
        Err(reason) => return GithubAuthResolution::Unavailable(metadata(account, Some(reason))),
    };
    let secret = String::from_utf8_lossy(&token_output.stdout)
        .trim()
        .to_owned();
    if secret.is_empty() {
        return GithubAuthResolution::Unavailable(metadata(
            account,
            Some(GithubAuthReason::GhLoggedOut),
        ));
    }

    GithubAuthResolution::Available(GithubCredential {
        secret,
        metadata: metadata(account, None),
    })
}

fn run_gh(arguments: &[&str]) -> Result<Output, GithubAuthReason> {
    let mut child = Command::new("gh")
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                GithubAuthReason::GhUnavailable
            } else {
                GithubAuthReason::GhCommandFailed
            }
        })?;

    let deadline = Instant::now() + GH_COMMAND_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|_| GithubAuthReason::GhCommandFailed);
            }
            Ok(None) if Instant::now() >= deadline => {
                terminate_child(&mut child);
                return Err(GithubAuthReason::GhTimedOut);
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(_) => {
                terminate_child(&mut child);
                return Err(GithubAuthReason::GhCommandFailed);
            }
        }
    }
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{anyhow, ensure};

    #[test]
    fn selects_dotcom_environment_variables_in_order() -> Result<()> {
        let environment = BTreeMap::from([
            ("GH_TOKEN".to_owned(), "first".to_owned()),
            ("GITHUB_TOKEN".to_owned(), "second".to_owned()),
        ]);
        let selected = environment_credential("github.com", &environment)
            .ok_or_else(|| anyhow!("expected environment credential"))?;
        ensure!(selected == (GithubAuthSource::GhToken, "first".to_owned()));
        Ok(())
    }

    #[test]
    fn selects_enterprise_environment_variables_without_cross_host_fallback() -> Result<()> {
        let environment = BTreeMap::from([
            ("GH_TOKEN".to_owned(), "dotcom".to_owned()),
            (
                "GITHUB_ENTERPRISE_TOKEN".to_owned(),
                "enterprise".to_owned(),
            ),
        ]);
        let selected = environment_credential("github.example.com", &environment)
            .ok_or_else(|| anyhow!("expected enterprise credential"))?;
        ensure!(
            selected
                == (
                    GithubAuthSource::GithubEnterpriseToken,
                    "enterprise".to_owned()
                )
        );
        Ok(())
    }

    #[test]
    fn ignores_empty_environment_values() -> Result<()> {
        let environment = BTreeMap::from([("GH_TOKEN".to_owned(), "  ".to_owned())]);
        ensure!(environment_credential("github.com", &environment).is_none());
        Ok(())
    }

    #[test]
    fn normalizes_configured_api_hosts() -> Result<()> {
        ensure!(github_host(None).map_err(|_| anyhow!("invalid default host"))? == "github.com");
        ensure!(
            github_host(Some("https://GitHub.Example.com/api/v3"))
                .map_err(|_| anyhow!("invalid enterprise host"))?
                == "github.example.com"
        );
        Ok(())
    }

    #[test]
    fn rejects_invalid_api_hosts() -> Result<()> {
        ensure!(github_host(Some("github.example.com")).is_err());
        ensure!(github_host(Some("https://")).is_err());
        Ok(())
    }

    #[test]
    fn safe_metadata_does_not_serialize_credential_material() -> Result<()> {
        let resolution = GithubAuthResolution::Available(GithubCredential {
            secret: "SHIPLOG_SECRET_SENTINEL".to_owned(),
            metadata: GithubAuthMetadata {
                source: GithubAuthSource::GhToken,
                host: "github.com".to_owned(),
                account: None,
                availability: GithubAuthAvailability::Ready,
                reason: None,
            },
        });
        let json = serde_json::to_string(resolution.metadata())?;
        ensure!(!json.contains("SHIPLOG_SECRET_SENTINEL"));
        ensure!(json.contains("gh_token"));
        ensure!(json.contains("github.com"));
        Ok(())
    }
}
