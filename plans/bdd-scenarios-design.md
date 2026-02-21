# BDD Scenario Design for shiplog ROADMAP Features

This document provides comprehensive BDD (Behavior-Driven Development) scenario designs for all features in the shiplog ROADMAP. Each scenario follows the Given/When/Then pattern and is designed to be implemented using the existing BDD framework from [`shiplog-testkit`](../crates/shiplog-testkit/src/bdd.rs).

## Table of Contents

1. [Framework Overview](#framework-overview)
2. [v0.2.x (Now) Features](#v02x-now-features)
3. [v0.3.x (Next) Features](#v03x-next-features)
4. [Later (Exploratory) Features](#later-exploratory-features)
5. [Reusable Step Library](#reusable-step-library)
6. [Implementation Guidelines](#implementation-guidelines)

---

## Framework Overview

The BDD framework in [`shiplog-testkit`](../crates/shiplog-testkit/src/bdd.rs) provides:

- **`ScenarioContext`**: Carries state through Given/When/Then steps with typed storage (strings, numbers, flags, paths, data)
- **`Scenario`**: Builder pattern for composing scenarios with descriptive steps
- **Assertion helpers**: `assert_present`, `assert_eq`, `assert_true`, `assert_false`, `assert_contains`, `assert_not_contains`
- **Fixture builders**: `EventBuilder`, `CoverageBuilder`, `WorkstreamsBuilder`

### Scenario Structure Template

```rust
let scenario = Scenario::new("Descriptive scenario name")
    .given("precondition 1", |ctx| { /* setup */ })
    .given("precondition 2", |ctx| { /* setup */ })
    .when("user action", |ctx| {
        // Execute action
        Ok(())
    })
    .then("expected outcome 1", |ctx| {
        // Verify outcome
        Ok(())
    })
    .then("expected outcome 2", |ctx| {
        // Verify outcome
        Ok(())
    });
```

---

## v0.2.x (Now) Features

### 1. Binary Releases via GitHub Actions

#### Primary User Scenarios

**Scenario 1.1: User downloads and installs binary for their platform**

```gherkin
Scenario: User downloads and installs binary for their platform
  Given a user on Windows 11 without Rust toolchain
  And the shiplog project has released version 0.2.0
  When they navigate to the GitHub releases page
  And they download the shiplog-0.2.0-x86_64-pc-windows-msvc.zip
  And they extract the archive to a directory in PATH
  Then they can run "shiplog --version" from the command line
  And the output shows "shiplog 0.2.0"
```

**Scenario 1.2: User downloads binary for macOS**

```gherkin
Scenario: User downloads binary for macOS
  Given a user on macOS (Apple Silicon) without Rust toolchain
  And the shiplog project has released version 0.2.0
  When they download the shiplog-0.2.0-aarch64-apple-darwin.tar.gz
  And they extract and move the binary to /usr/local/bin
  Then they can run "shiplog --version" from the command line
  And the output shows "shiplog 0.2.0"
```

**Scenario 1.3: User downloads binary for Linux**

```gherkin
Scenario: User downloads binary for Linux
  Given a user on Linux (x86_64) without Rust toolchain
  And the shiplog project has released version 0.2.0
  When they download the shiplog-0.2.0-x86_64-unknown-linux-gnu.tar.gz
  And they extract and move the binary to ~/.local/bin
  Then they can run "shiplog --version" from the command line
  And the output shows "shiplog 0.2.0"
```

#### Edge Cases and Error Scenarios

**Scenario 1.4: Binary signature verification fails**

```gherkin
Scenario: Binary signature verification fails
  Given a user has downloaded a shiplog binary
  And the binary has been tampered with
  When they attempt to verify the signature
  Then the verification should fail
  And a clear error message should indicate signature mismatch
  And the user should be warned not to run the binary
```

**Scenario 1.5: Binary for unsupported platform**

```gherkin
Scenario: Binary for unsupported platform
  Given a user on an unsupported platform (e.g., ARM Linux)
  When they navigate to the GitHub releases page
  Then no binary should be available for their platform
  And instructions should suggest building from source
```

**Scenario 1.6: Binary execution permission denied**

```gherkin
Scenario: Binary execution permission denied
  Given a user has downloaded and extracted a Linux binary
  And the binary does not have execute permissions
  When they attempt to run "shiplog --version"
  Then they should receive a permission denied error
  And documentation should guide them to run "chmod +x shiplog"
```

#### Integration Scenarios

**Scenario 1.7: Binary works with all existing features**

```gherkin
Scenario: Binary works with all existing features
  Given a user has installed the shiplog binary
  And they have a GitHub token configured
  When they run "shiplog collect --user alice --since 2025-01-01"
  Then events should be collected successfully
  When they run "shiplog cluster"
  Then workstreams should be generated
  When they run "shiplog render"
  Then a packet markdown file should be created
```

#### Performance/Load Scenarios

**Scenario 1.8: Binary startup time**

```gherkin
Scenario: Binary startup time is acceptable
  Given a user has installed the shiplog binary
  When they run "shiplog --help"
  Then the command should complete within 100ms
```

---

### 2. Local Git Ingest Adapter

#### Primary User Scenarios

**Scenario 2.1: User ingests commits from local git repository**

```gherkin
Scenario: User ingests commits from local git repository
  Given a user has a local git repository at /path/to/project
  And the repository has commits authored by the user
  When they run "shiplog collect --source git --repo /path/to/project --since 2025-01-01"
  Then events should be generated from local git commits
  And each event should have SourceSystem::LocalGit
  And events should include commit hash, message, author, and timestamp
```

**Scenario 2.2: User filters commits by date range**

```gherkin
Scenario: User filters commits by date range
  Given a user has a local git repository
  And the repository has commits spanning multiple months
  When they run "shiplog collect --source git --repo /path/to/project --since 2025-01-01 --until 2025-01-31"
  Then only commits within the date range should be included
  And commits outside the range should be excluded
```

**Scenario 2.3: User ingests commits with author filtering**

```gherkin
Scenario: User ingests commits with author filtering
  Given a user has a local git repository
  And the repository has commits from multiple authors
  When they run "shiplog collect --source git --repo /path/to/project --author alice@example.com"
  Then only commits authored by the specified email should be included
  And commits from other authors should be excluded
```

#### Edge Cases and Error Scenarios

**Scenario 2.4: Repository path does not exist**

```gherkin
Scenario: Repository path does not exist
  Given a user specifies a non-existent repository path
  When they run "shiplog collect --source git --repo /nonexistent/path"
  Then the command should fail with a clear error message
  And the error should indicate the path does not exist
```

**Scenario 2.5: Path exists but is not a git repository**

```gherkin
Scenario: Path exists but is not a git repository
  Given a user specifies a path that exists but is not a git repository
  When they run "shiplog collect --source git --repo /path/to/non-git-dir"
  Then the command should fail with a clear error message
  And the error should indicate the directory is not a git repository
```

**Scenario 2.6: Repository has no commits**

```gherkin
Scenario: Repository has no commits
  Given a user has a newly initialized git repository with no commits
  When they run "shiplog collect --source git --repo /path/to/empty-repo"
  Then no events should be generated
  And a warning should indicate no commits were found
```

**Scenario 2.7: Repository has no matching commits in date range**

```gherkin
Scenario: Repository has no matching commits in date range
  Given a user has a git repository with commits
  And they specify a date range before any commits exist
  When they run "shiplog collect --source git --repo /path/to/repo --since 2000-01-01 --until 2000-12-31"
  Then no events should be generated
  And a warning should indicate no commits were found in the range
```

**Scenario 2.8: Commit with invalid author format**

```gherkin
Scenario: Commit with invalid author format
  Given a user has a git repository
  And the repository contains a commit with malformed author information
  When they run "shiplog collect --source git --repo /path/to/repo"
  Then the commit should be skipped
  And a warning should indicate the invalid commit format
```

#### Integration Scenarios

**Scenario 2.9: Local git events merge with GitHub events**

```gherkin
Scenario: Local git events merge with GitHub events
  Given a user has collected events from GitHub API
  And they also have a local git repository
  When they run "shiplog collect --source git --repo /path/to/repo"
  Then local git events should be merged with existing GitHub events
  And events from both sources should appear in the ledger
  And coverage manifest should include both sources
```

**Scenario 2.10: Local git events cluster into workstreams**

```gherkin
Scenario: Local git events cluster into workstreams
  Given a user has collected local git events
  When they run "shiplog cluster"
  Then workstreams should be generated from local git events
  And workstreams should group related commits
```

**Scenario 2.11: Local git events render in packet**

```gherkin
Scenario: Local git events render in packet
  Given a user has collected local git events and generated workstreams
  When they run "shiplog render"
  Then the packet should include local git events
  And events should display commit hash and message
  And source should be indicated as "local git"
```

#### Performance/Load Scenarios

**Scenario 2.12: Repository with thousands of commits**

```gherkin
Scenario: Repository with thousands of commits
  Given a user has a git repository with 10,000 commits
  When they run "shiplog collect --source git --repo /path/to/large-repo --since 2025-01-01"
  Then collection should complete within reasonable time (< 30 seconds)
  And memory usage should remain bounded (< 500MB)
```

**Scenario 2.13: Repository with large commit messages**

```gherkin
Scenario: Repository with large commit messages
  Given a user has a git repository with commits containing large messages (> 10KB)
  When they run "shiplog collect --source git --repo /path/to/repo"
  Then large commit messages should be handled correctly
  And the collection should not fail or hang
```

---

### 3. Improved Packet Formatting

#### Primary User Scenarios

**Scenario 3.1: User renders packet with improved structure**

```gherkin
Scenario: User renders packet with improved structure
  Given a user has collected events and generated workstreams
  When they run "shiplog render"
  Then the packet should have clear section headers
  And sections should be ordered logically (summary, workstreams, receipts, coverage)
  And each section should be visually distinct
```

**Scenario 3.2: User configures section ordering**

```gherkin
Scenario: User configures section ordering
  Given a user has a shiplog config file
  And the config specifies custom section order: coverage, workstreams, summary
  When they run "shiplog render"
  Then sections should appear in the configured order
  And the default order should be overridden
```

**Scenario 3.3: User renders packet with cleaner receipt presentation**

```gherkin
Scenario: User renders packet with cleaner receipt presentation
  Given a user has curated workstreams with selected receipts
  When they run "shiplog render"
  Then each receipt should be presented in a consistent format
  And receipts should include event type, title, date, and link
  And receipts should be grouped by workstream
```

**Scenario 3.4: User renders packet with improved Markdown structure**

```gherkin
Scenario: User renders packet with improved Markdown structure
  Given a user has collected events and generated workstreams
  When they run "shiplog render"
  Then the packet should use proper Markdown headings (H1, H2, H3)
  And lists should be properly formatted
  And code blocks should use proper syntax highlighting
  And tables should be used for structured data where appropriate
```

#### Edge Cases and Error Scenarios

**Scenario 3.5: Packet with no workstreams**

```gherkin
Scenario: Packet with no workstreams
  Given a user has collected events but no workstreams were generated
  When they run "shiplog render"
  Then a packet should still be generated
  And a message should indicate no workstreams were found
  And events should be listed in a raw format
```

**Scenario 3.6: Packet with empty workstreams**

```gherkin
Scenario: Packet with empty workstreams
  Given a user has workstreams with no events or receipts
  When they run "shiplog render"
  Then empty workstreams should be indicated
  And the packet should still render successfully
```

**Scenario 3.7: Invalid section ordering configuration**

```gherkin
Scenario: Invalid section ordering configuration
  Given a user has a config file with invalid section names
  When they run "shiplog render"
  Then the command should fail with a clear error
  And the error should list valid section names
```

#### Integration Scenarios

**Scenario 3.8: Packet formatting works with all event sources**

```gherkin
Scenario: Packet formatting works with all event sources
  Given a user has collected events from GitHub, local git, and manual entries
  And they have generated workstreams
  When they run "shiplog render"
  Then the packet should correctly format events from all sources
  And source indicators should be consistent
```

**Scenario 3.9: Packet formatting preserves redaction**

```gherkin
Scenario: Packet formatting preserves redaction
  Given a user has collected events with sensitive information
  And they are rendering with a redaction profile
  When they run "shiplog render --redact public"
  Then the formatted packet should not contain sensitive data
  And redacted fields should be clearly marked
```

#### Performance/Load Scenarios

**Scenario 3.10: Packet with hundreds of receipts**

```gherkin
Scenario: Packet with hundreds of receipts
  Given a user has workstreams with 500 total receipts
  When they run "shiplog render"
  Then rendering should complete within reasonable time (< 5 seconds)
  And the output file size should be reasonable (< 1MB)
```

---

### 4. Cache Improvements

#### Primary User Scenarios

**Scenario 4.1: User configures cache TTL**

```gherkin
Scenario: User configures cache TTL
  Given a user has a shiplog config file
  And the config specifies cache_ttl of 7 days
  When they run "shiplog collect"
  Then cache entries should expire after 7 days
  And entries older than 7 days should be considered stale
```

**Scenario 4.2: User configures cache size limit**

```gherkin
Scenario: User configures cache size limit
  Given a user has a shiplog config file
  And the config specifies cache_size_limit of 100MB
  When the cache grows beyond 100MB
  Then oldest entries should be evicted to maintain the limit
```

**Scenario 4.3: User inspects cache contents**

```gherkin
Scenario: User inspects cache contents
  Given a user has collected events with caching enabled
  When they run "shiplog cache stats"
  Then the output should show total entries
  And the output should show valid entries
  And the output should show expired entries
  And the output should show cache size on disk
```

**Scenario 4.4: User clears cache**

```gherkin
Scenario: User clears cache
  Given a user has a populated cache
  When they run "shiplog cache clear"
  Then all cache entries should be deleted
  And the cache file should be empty
  And a confirmation message should be displayed
```

**Scenario 4.5: User cleans up expired cache entries**

```gherkin
Scenario: User cleans up expired cache entries
  Given a user has a cache with expired entries
  When they run "shiplog cache cleanup"
  Then expired entries should be deleted
  And valid entries should be preserved
  And the number of deleted entries should be reported
```

#### Edge Cases and Error Scenarios

**Scenario 4.6: Cache file is corrupted**

```gherkin
Scenario: Cache file is corrupted
  Given a user has a corrupted cache database file
  When they run "shiplog collect"
  Then the cache should be recreated from scratch
  And a warning should indicate the cache was reset
  And collection should proceed normally
```

**Scenario 4.7: Cache directory is not writable**

```gherkin
Scenario: Cache directory is not writable
  Given a user does not have write permissions to the cache directory
  When they run "shiplog collect"
  Then caching should be disabled for this run
  And a warning should indicate caching is disabled
  And collection should proceed without caching
```

**Scenario 4.8: Cache with zero TTL**

```gherkin
Scenario: Cache with zero TTL
  Given a user has configured cache_ttl of 0
  When they run "shiplog collect"
  Then cache entries should be considered immediately expired
  And no cache hits should occur
  And caching should effectively be disabled
```

#### Integration Scenarios

**Scenario 4.9: Cache works across multiple runs**

```gherkin
Scenario: Cache works across multiple runs
  Given a user runs "shiplog collect" with caching enabled
  And they run the same command again immediately
  Then the second run should use cached data
  And no API calls should be made for cached data
  And the second run should complete faster
```

**Scenario 4.10: Cache works with multiple sources**

```gherkin
Scenario: Cache works with multiple sources
  Given a user collects from GitHub and local git
  And both sources use caching
  When they run "shiplog collect"
  Then cache should be used for GitHub API responses
  And local git data should not need caching
  And cache stats should reflect GitHub cache usage
```

#### Performance/Load Scenarios

**Scenario 4.11: Cache with thousands of entries**

```gherkin
Scenario: Cache with thousands of entries
  Given a user has a cache with 10,000 entries
  When they run "shiplog cache stats"
  Then the command should complete quickly (< 1 second)
  And the output should be accurate
```

**Scenario 4.12: Cache cleanup performance**

```gherkin
Scenario: Cache cleanup performance
  Given a user has a cache with 5,000 expired entries
  When they run "shiplog cache cleanup"
  Then cleanup should complete within reasonable time (< 5 seconds)
  And all expired entries should be removed
```

---

## v0.3.x (Next) Features

### 5. GitLab Ingest Adapter

#### Primary User Scenarios

**Scenario 5.1: User ingests merge requests from GitLab.com**

```gherkin
Scenario: User ingests merge requests from GitLab.com
  Given a user has a GitLab.com account
  And they have a personal access token configured
  When they run "shiplog collect --source gitlab --user alice --since 2025-01-01"
  Then merge requests should be collected from GitLab.com
  And each event should have SourceSystem::Other("gitlab")
  And events should include MR number, title, state, and timestamp
```

**Scenario 5.2: User ingests from self-hosted GitLab instance**

```gherkin
Scenario: User ingests from self-hosted GitLab instance
  Given a user has access to a self-hosted GitLab instance at gitlab.company.com
  And they have configured the instance URL and token
  When they run "shiplog collect --source gitlab --instance gitlab.company.com --user alice --since 2025-01-01"
  Then merge requests should be collected from the self-hosted instance
  And the API base URL should be correctly configured
```

**Scenario 5.3: User ingests GitLab reviews**

```gherkin
Scenario: User ingests GitLab reviews
  Given a user has configured GitLab collection with review inclusion
  When they run "shiplog collect --source gitlab --user alice --include-reviews"
  Then merge request reviews should be collected
  And review events should include the MR being reviewed
  And review events should include the reviewer and timestamp
```

**Scenario 5.4: User filters GitLab MRs by state**

```gherkin
Scenario: User filters GitLab MRs by state
  Given a user has GitLab MRs in various states (opened, merged, closed)
  When they run "shiplog collect --source gitlab --user alice --state merged"
  Then only merged MRs should be collected
  And opened and closed MRs should be excluded
```

#### Edge Cases and Error Scenarios

**Scenario 5.5: Invalid GitLab instance URL**

```gherkin
Scenario: Invalid GitLab instance URL
  Given a user specifies an invalid GitLab instance URL
  When they run "shiplog collect --source gitlab --instance invalid-url"
  Then the command should fail with a clear error message
  And the error should indicate the URL is invalid
```

**Scenario 5.6: GitLab authentication failure**

```gherkin
Scenario: GitLab authentication failure
  Given a user has an invalid GitLab token
  When they run "shiplog collect --source gitlab --user alice"
  Then the command should fail with an authentication error
  And the error should indicate the token is invalid or expired
```

**Scenario 5.7: GitLab API rate limit exceeded**

```gherkin
Scenario: GitLab API rate limit exceeded
  Given a user has exceeded the GitLab API rate limit
  When they run "shiplog collect --source gitlab --user alice"
  Then the command should fail with a rate limit error
  And the error should indicate when the limit will reset
```

**Scenario 5.8: GitLab project is private and inaccessible**

```gherkin
Scenario: GitLab project is private and inaccessible
  Given a user attempts to collect from a private GitLab project
  And they do not have access to the project
  When they run "shiplog collect --source gitlab --user alice"
  Then the command should fail with an access error
  And the error should indicate the project is inaccessible
```

#### Integration Scenarios

**Scenario 5.9: GitLab events merge with GitHub events**

```gherkin
Scenario: GitLab events merge with GitHub events
  Given a user has collected events from GitHub
  And they also collect events from GitLab
  Then events from both sources should be merged into a single ledger
  And coverage manifest should include both sources
  And workstreams can contain events from both sources
```

**Scenario 5.10: GitLab events cluster into workstreams**

```gherkin
Scenario: GitLab events cluster into workstreams
  Given a user has collected GitLab MRs and reviews
  When they run "shiplog cluster"
  Then workstreams should be generated from GitLab events
  And related MRs should be grouped together
```

**Scenario 5.11: GitLab events render in packet**

```gherkin
Scenario: GitLab events render in packet
  Given a user has collected GitLab events and generated workstreams
  When they run "shiplog render"
  Then the packet should include GitLab MRs
  And MRs should link to the GitLab instance
  And source should be indicated as "gitlab"
```

#### Performance/Load Scenarios

**Scenario 5.12: GitLab collection with thousands of MRs**

```gherkin
Scenario: GitLab collection with thousands of MRs
  Given a user has 2,000 MRs on GitLab
  When they run "shiplog collect --source gitlab --user alice --since 2025-01-01"
  Then collection should complete within reasonable time (< 60 seconds)
  And memory usage should remain bounded (< 500MB)
```

---

### 6. Jira/Linear Ingest Adapter

#### Primary User Scenarios

**Scenario 6.1: User ingests Jira issues**

```gherkin
Scenario: User ingests Jira issues
  Given a user has a Jira account
  And they have configured Jira instance URL and API token
  When they run "shiplog collect --source jira --user alice --since 2025-01-01"
  Then Jira issues should be collected
  And each event should have SourceSystem::Other("jira")
  And events should include issue key, summary, status, and timestamp
```

**Scenario 6.2: User ingests Linear issues**

```gherkin
Scenario: User ingests Linear issues
  Given a user has a Linear account
  And they have configured Linear API key
  When they run "shiplog collect --source linear --user alice --since 2025-01-01"
  Then Linear issues should be collected
  And each event should have SourceSystem::Other("linear")
  And events should include issue ID, title, status, and timestamp
```

**Scenario 6.3: User filters Jira issues by status**

```gherkin
Scenario: User filters Jira issues by status
  Given a user has Jira issues in various statuses
  When they run "shiplog collect --source jira --user alice --status Done"
  Then only issues with "Done" status should be collected
  And issues in other statuses should be excluded
```

**Scenario 6.4: User filters Linear issues by project**

```gherkin
Scenario: User filters Linear issues by project
  Given a user has Linear issues across multiple projects
  When they run "shiplog collect --source linear --user alice --project PROJ-123"
  Then only issues from the specified project should be collected
```

#### Edge Cases and Error Scenarios

**Scenario 6.5: Invalid Jira instance URL**

```gherkin
Scenario: Invalid Jira instance URL
  Given a user specifies an invalid Jira instance URL
  When they run "shiplog collect --source jira --instance invalid-url"
  Then the command should fail with a clear error message
  And the error should indicate the URL is invalid
```

**Scenario 6.6: Jira authentication failure**

```gherkin
Scenario: Jira authentication failure
  Given a user has an invalid Jira API token
  When they run "shiplog collect --source jira --user alice"
  Then the command should fail with an authentication error
  And the error should indicate the token is invalid
```

**Scenario 6.7: Linear API key invalid**

```gherkin
Scenario: Linear API key invalid
  Given a user has an invalid Linear API key
  When they run "shiplog collect --source linear --user alice"
  Then the command should fail with an authentication error
  And the error should indicate the API key is invalid
```

**Scenario 6.8: Issue with missing required fields**

```gherkin
Scenario: Issue with missing required fields
  Given a user has a Jira issue with missing summary or status
  When they run "shiplog collect --source jira --user alice"
  Then the issue should be skipped
  And a warning should indicate the missing fields
```

#### Integration Scenarios

**Scenario 6.9: Jira issues correlate with GitHub PRs**

```gherkin
Scenario: Jira issues correlate with GitHub PRs
  Given a user has collected GitHub PRs
  And PR titles contain Jira issue keys (e.g., "PROJ-123: Fix bug")
  When they also collect Jira issues
  Then the system should attempt to correlate PRs with issues
  And workstreams may group related PRs and issues together
```

**Scenario 6.10: Linear issues appear in packet**

```gherkin
Scenario: Linear issues appear in packet
  Given a user has collected Linear issues and generated workstreams
  When they run "shiplog render"
  Then the packet should include Linear issues
  And issues should link to the Linear web interface
  And source should be indicated as "linear"
```

#### Performance/Load Scenarios

**Scenario 6.11: Jira collection with hundreds of issues**

```gherkin
Scenario: Jira collection with hundreds of issues
  Given a user has 500 Jira issues
  When they run "shiplog collect --source jira --user alice --since 2025-01-01"
  Then collection should complete within reasonable time (< 30 seconds)
```

---

### 7. Multi-Source Merging

#### Primary User Scenarios

**Scenario 7.1: User merges events from multiple sources**

```gherkin
Scenario: User merges events from multiple sources
  Given a user has collected events from GitHub
  And they have collected events from local git
  And they have collected manual events
  When they run "shiplog merge --sources github,local_git,manual"
  Then all events should be merged into a single ledger
  And events should be deduplicated by ID
  And events should be sorted by timestamp
```

**Scenario 7.2: User merges with unified coverage tracking**

```gherkin
Scenario: User merges with unified coverage tracking
  Given a user has collected events from multiple sources
  When they merge the sources
  Then the coverage manifest should include all sources
  And completeness should be calculated across all sources
  And warnings should be aggregated from all sources
```

**Scenario 7.3: User merges events from same source type**

```gherkin
Scenario: User merges events from same source type
  Given a user has collected events from GitHub for two different repos
  When they merge the sources
  Then events from both repos should be included
  And duplicate events (same PR) should be deduplicated
```

#### Edge Cases and Error Scenarios

**Scenario 7.4: Conflicting events from different sources**

```gherkin
Scenario: Conflicting events from different sources
  Given a user has collected the same event from GitHub and local git
  And the events have different metadata
  When they merge the sources
  Then one event should be chosen as authoritative
  And a warning should indicate the conflict was resolved
```

**Scenario 7.5: Merge with no events**

```gherkin
Scenario: Merge with no events
  Given a user attempts to merge with no collected events
  When they run "shiplog merge"
  Then the command should fail with a clear error
  And the error should indicate no events are available to merge
```

**Scenario 7.6: Merge with incompatible event types**

```gherkin
Scenario: Merge with incompatible event types
  Given a user has collected events from a source with incompatible schema
  When they attempt to merge
  Then incompatible events should be skipped
  And a warning should indicate the skipped events
```

#### Integration Scenarios

**Scenario 7.7: Multi-source events cluster together**

```gherkin
Scenario: Multi-source events cluster together
  Given a user has merged events from GitHub, GitLab, and Jira
  When they run "shiplog cluster"
  Then workstreams should include events from all sources
  And clustering should consider event context across sources
```

**Scenario 7.8: Multi-source packet renders correctly**

```gherkin
Scenario: Multi-source packet renders correctly
  Given a user has merged events from multiple sources
  And they have generated workstreams
  When they run "shiplog render"
  Then the packet should include events from all sources
  And each event should indicate its source
```

#### Performance/Load Scenarios

**Scenario 7.9: Merge with thousands of events**

```gherkin
Scenario: Merge with thousands of events
  Given a user has 5,000 events across 5 sources
  When they run "shiplog merge"
  Then merging should complete within reasonable time (< 10 seconds)
  And memory usage should remain bounded
```

---

### 8. Configurable Packet Templates

#### Primary User Scenarios

**Scenario 8.1: User defines custom packet template**

```gherkin
Scenario: User defines custom packet template
  Given a user has a custom Jinja2 template file at templates/custom.md
  And the template defines custom packet structure
  When they run "shiplog render --template templates/custom.md"
  Then the packet should be rendered using the custom template
  And the output should match the template structure
```

**Scenario 8.2: User template includes custom variables**

```gherkin
Scenario: User template includes custom variables
  Given a user has a custom template with variables like {{ user_name }}, {{ company }}
  And they have configured these variables in their config
  When they run "shiplog render --template templates/custom.md"
  Then the template variables should be substituted with configured values
```

**Scenario 8.3: User template includes conditional sections**

```gherkin
Scenario: User template includes conditional sections
  Given a user has a custom template with conditional sections
  And the template shows a section only if {{ show_details }} is true
  When they run "shiplog render --template templates/custom.md --show-details"
  Then the conditional section should be included
```

**Scenario 8.4: User template includes loops over workstreams**

```gherkin
Scenario: User template includes loops over workstreams
  Given a user has a custom template with {% for ws in workstreams %}
  And they have multiple workstreams
  When they run "shiplog render --template templates/custom.md"
  Then the template should iterate over all workstreams
  And each workstream should be rendered according to the template
```

#### Edge Cases and Error Scenarios

**Scenario 8.5: Template file does not exist**

```gherkin
Scenario: Template file does not exist
  Given a user specifies a non-existent template file
  When they run "shiplog render --template nonexistent.md"
  Then the command should fail with a clear error message
  And the error should indicate the template file was not found
```

**Scenario 8.6: Template has syntax errors**

```gherkin
Scenario: Template has syntax errors
  Given a user has a template with Jinja2 syntax errors
  When they run "shiplog render --template broken.md"
  Then the command should fail with a clear error message
  And the error should indicate the syntax error location
```

**Scenario 8.7: Template references undefined variable**

```gherkin
Scenario: Template references undefined variable
  Given a user has a template that references {{ undefined_var }}
  When they run "shiplog render --template template.md"
  Then the command should fail with a clear error message
  And the error should indicate the undefined variable
```

#### Integration Scenarios

**Scenario 8.8: Custom template works with all event sources**

```gherkin
Scenario: Custom template works with all event sources
  Given a user has collected events from multiple sources
  And they have a custom template
  When they run "shiplog render --template custom.md"
  Then the template should correctly render events from all sources
  And source-specific formatting should work
```

**Scenario 8.9: Custom template preserves redaction**

```gherkin
Scenario: Custom template preserves redaction
  Given a user has collected events with sensitive information
  And they are rendering with a redaction profile
  And they have a custom template
  When they run "shiplog render --template custom.md --redact public"
  Then the rendered packet should not contain sensitive data
  And redaction should work with the custom template
```

#### Performance/Load Scenarios

**Scenario 8.10: Complex template with many workstreams**

```gherkin
Scenario: Complex template with many workstreams
  Given a user has 50 workstreams
  And they have a complex custom template
  When they run "shiplog render --template complex.md"
  Then rendering should complete within reasonable time (< 10 seconds)
```

---

### 9. LLM Clustering as Default Feature

#### Primary User Scenarios

**Scenario 9.1: User uses LLM clustering without feature flag**

```gherkin
Scenario: User uses LLM clustering without feature flag
  Given a user has configured an LLM API key
  And they have collected events
  When they run "shiplog cluster"
  Then LLM clustering should be used by default
  And no feature flag should be required
  And workstreams should be generated using LLM analysis
```

**Scenario 9.2: User falls back to repo-based clustering on LLM failure**

```gherkin
Scenario: User falls back to repo-based clustering on LLM failure
  Given a user has configured LLM clustering
  And the LLM API is temporarily unavailable
  When they run "shiplog cluster"
  Then the system should fall back to repo-based clustering
  And a warning should indicate the fallback occurred
  And workstreams should still be generated
```

**Scenario 9.3: User configures LLM clustering parameters**

```gherkin
Scenario: User configures LLM clustering parameters
  Given a user has a config file with LLM settings
  And the config specifies max_workstreams: 10
  And the config specifies model: "gpt-4"
  When they run "shiplog cluster"
  Then LLM clustering should use the configured parameters
  And the specified model should be used
  And workstreams should be limited to the configured maximum
```

#### Edge Cases and Error Scenarios

**Scenario 9.4: LLM API key not configured**

```gherkin
Scenario: LLM API key not configured
  Given a user has not configured an LLM API key
  When they run "shiplog cluster"
  Then the system should fall back to repo-based clustering
  And a warning should indicate LLM is not configured
```

**Scenario 9.5: LLM returns invalid response**

```gherkin
Scenario: LLM returns invalid response
  Given a user has configured LLM clustering
  And the LLM returns a malformed response
  When they run "shiplog cluster"
  Then the system should fall back to repo-based clustering
  And a warning should indicate the LLM response was invalid
```

**Scenario 9.6: LLM API rate limit exceeded**

```gherkin
Scenario: LLM API rate limit exceeded
  Given a user has exceeded the LLM API rate limit
  When they run "shiplog cluster"
  Then the system should fall back to repo-based clustering
  And a warning should indicate the rate limit was exceeded
```

#### Integration Scenarios

**Scenario 9.7: LLM clustering works with multi-source events**

```gherkin
Scenario: LLM clustering works with multi-source events
  Given a user has collected events from GitHub, GitLab, and Jira
  When they run "shiplog cluster"
  Then LLM clustering should consider events from all sources
  And workstreams may group events from different sources together
```

**Scenario 9.8: LLM clustering preserves user curation**

```gherkin
Scenario: LLM clustering preserves user curation
  Given a user has curated workstreams with custom titles
  When they refresh with new events and run "shiplog cluster"
  Then the curated workstream titles should be preserved
  And new events should be added to appropriate workstreams
```

#### Performance/Load Scenarios

**Scenario 9.9: LLM clustering with many events**

```gherkin
Scenario: LLM clustering with many events
  Given a user has 1,000 events
  When they run "shiplog cluster"
  Then clustering should complete within reasonable time (< 60 seconds)
  And the system should chunk events if needed
```

---

## Later (Exploratory) Features

### 10. Team Aggregation Mode

#### Primary User Scenarios

**Scenario 10.1: User generates team-level shipping summary**

```gherkin
Scenario: User generates team-level shipping summary
  Given a user is a team lead
  And they have access to multiple team members' shiplog ledgers
  When they run "shiplog team-aggregate --members alice,bob,charlie --since 2025-01-01"
  Then a team-level packet should be generated
  And the packet should include sections for each team member
  And the packet should include a team summary section
```

**Scenario 10.2: User aggregates team with configurable sections**

```gherkin
Scenario: User aggregates team with configurable sections
  Given a user is generating a team packet
  And they want to include only workstreams and coverage
  When they run "shiplog team-aggregate --members alice,bob --sections workstreams,coverage"
  Then the team packet should include only the specified sections
  And other sections should be excluded
```

**Scenario 10.3: User aggregates team with member aliases**

```gherkin
Scenario: User aggregates team with member aliases
  Given a user has team members with different display names
  And they configure member aliases in a config file
  When they run "shiplog team-aggregate --config team.yaml"
  Then the team packet should use the configured aliases
  And member identities should be consistent
```

#### Edge Cases and Error Scenarios

**Scenario 10.4: Member ledger not found**

```gherkin
Scenario: Member ledger not found
  Given a user specifies a team member
  And that member's ledger does not exist
  When they run "shiplog team-aggregate --members alice,nonexistent"
  Then a warning should indicate the missing ledger
  And the packet should be generated for available members
```

**Scenario 10.5: Member ledger has incompatible version**

```gherkin
Scenario: Member ledger has incompatible version
  Given a user specifies a team member
  And that member's ledger uses an incompatible schema version
  When they run "shiplog team-aggregate --members alice,bob"
  Then a warning should indicate the incompatible ledger
  And that member's data should be excluded
```

#### Integration Scenarios

**Scenario 10.6: Team aggregation uses custom template**

```gherkin
Scenario: Team aggregation uses custom template
  Given a user has a custom team template
  When they run "shiplog team-aggregate --template team.md"
  Then the team packet should use the custom template
  And the template should render all team members
```

#### Performance/Load Scenarios

**Scenario 10.7: Team aggregation with many members**

```gherkin
Scenario: Team aggregation with many members
  Given a user has a team of 20 members
  When they run "shiplog team-aggregate --members all"
  Then aggregation should complete within reasonable time (< 30 seconds)
```

---

### 11. Continuous/Cron Mode

#### Primary User Scenarios

**Scenario 11.1: User runs shiplog on schedule**

```gherkin
Scenario: User runs shiplog on schedule
  Given a user has configured cron mode
  And the config specifies a schedule of "0 0 * * 0" (weekly)
  When the cron schedule triggers
  Then shiplog should collect new events since the last run
  And new events should be appended to the existing ledger
  And a new packet should be generated
```

**Scenario 11.2: User configures incremental collection**

```gherkin
Scenario: User configures incremental collection
  Given a user has enabled cron mode
  And the config specifies incremental: true
  When the cron schedule triggers
  Then only new events since the last run should be collected
  And the existing ledger should be preserved
```

**Scenario 11.3: User configures full collection on schedule**

```gherkin
Scenario: User configures full collection on schedule
  Given a user has enabled cron mode
  And the config specifies incremental: false
  And the config specifies a date range
  When the cron schedule triggers
  Then a full collection for the date range should be performed
  And the ledger should be replaced with new data
```

#### Edge Cases and Error Scenarios

**Scenario 11.4: Cron run fails**

```gherkin
Scenario: Cron run fails
  Given a user has enabled cron mode
  And a scheduled run fails due to API error
  Then the failure should be logged
  And the next scheduled run should still attempt
  And a notification should be sent if configured
```

**Scenario 11.5: No new events since last run**

```gherkin
Scenario: No new events since last run
  Given a user has enabled cron mode with incremental collection
  And no new events have occurred since the last run
  When the cron schedule triggers
  Then the collection should complete with no new events
  And a new packet should not be generated
  And a log message should indicate no changes
```

#### Integration Scenarios

**Scenario 11.6: Cron mode works with multi-source**

```gherkin
Scenario: Cron mode works with multi-source
  Given a user has configured cron mode
  And they have multiple sources configured
  When the cron schedule triggers
  Then all sources should be checked for new events
  And new events from all sources should be appended
```

#### Performance/Load Scenarios

**Scenario 11.7: Cron mode with large incremental update**

```gherkin
Scenario: Cron mode with large incremental update
  Given a user has enabled cron mode
  And 500 new events have occurred since the last run
  When the cron schedule triggers
  Then collection should complete within reasonable time (< 60 seconds)
```

---

### 12. TUI Workstream Editor

#### Primary User Scenarios

**Scenario 12.1: User opens TUI workstream editor**

```gherkin
Scenario: User opens TUI workstream editor
  Given a user has generated workstreams
  When they run "shiplog edit workstreams"
  Then a terminal UI should open
  And workstreams should be displayed in a list
  And the user should be able to navigate with keyboard
```

**Scenario 12.2: User renames a workstream in TUI**

```gherkin
Scenario: User renames a workstream in TUI
  Given a user has the TUI workstream editor open
  And they select a workstream
  When they press the rename key (e.g., 'r')
  And they enter a new title
  Then the workstream title should be updated
  And the change should be reflected in the UI
```

**Scenario 12.3: User adds a summary to a workstream in TUI**

```gherkin
Scenario: User adds a summary to a workstream in TUI
  Given a user has the TUI workstream editor open
  And they select a workstream
  When they press the edit summary key (e.g., 's')
  And they enter a summary
  Then the workstream summary should be updated
  And the change should be reflected in the UI
```

**Scenario 12.4: User selects receipts in TUI**

```gherkin
Scenario: User selects receipts in TUI
  Given a user has the TUI workstream editor open
  And they select a workstream
  When they press the edit receipts key (e.g., 'e')
  Then a list of events in the workstream should be displayed
  And the user can toggle events as receipts
  And selected receipts should be highlighted
```

**Scenario 12.5: User saves changes in TUI**

```gherkin
Scenario: User saves changes in TUI
  Given a user has made changes in the TUI editor
  When they press the save key (e.g., 'Ctrl+S')
  Then the workstreams file should be updated
  And a confirmation message should be displayed
  And the TUI should remain open for further editing
```

**Scenario 12.6: User exits TUI without saving**

```gherkin
Scenario: User exits TUI without saving
  Given a user has made changes in the TUI editor
  When they press the quit key (e.g., 'q')
  Then a prompt should ask to save changes
  If they choose not to save:
    Then changes should be discarded
    And the TUI should close
```

#### Edge Cases and Error Scenarios

**Scenario 12.7: TUI with no workstreams**

```gherkin
Scenario: TUI with no workstreams
  Given a user has no workstreams generated
  When they run "shiplog edit workstreams"
  Then the TUI should display a message indicating no workstreams
  And the user should be prompted to generate workstreams first
```

**Scenario 12.8: TUI with very long workstream title**

```gherkin
Scenario: TUI with very long workstream title
  Given a user has a workstream with a very long title (> 200 characters)
  When they open the TUI editor
  Then the title should be truncated in the list view
  And the full title should be visible in the edit view
```

#### Integration Scenarios

**Scenario 12.9: TUI changes reflected in rendered packet**

```gherkin
Scenario: TUI changes reflected in rendered packet
  Given a user has edited workstreams in the TUI
  And they have saved the changes
  When they run "shiplog render"
  Then the packet should reflect the TUI edits
  And workstream titles and summaries should match the TUI
```

#### Performance/Load Scenarios

**Scenario 12.10: TUI with many workstreams**

```gherkin
Scenario: TUI with many workstreams
  Given a user has 100 workstreams
  When they run "shiplog edit workstreams"
  Then the TUI should open within reasonable time (< 2 seconds)
  And navigation should remain responsive
```

---

### 13. Web Viewer

#### Primary User Scenarios

**Scenario 13.1: User launches web viewer**

```gherkin
Scenario: User launches web viewer
  Given a user has a rendered packet
  When they run "shiplog web-serve"
  Then a web server should start
  And the packet should be accessible in a browser
  And the server should listen on a default port (e.g., 8080)
```

**Scenario 13.2: User navigates workstreams in web viewer**

```gherkin
Scenario: User navigates workstreams in web viewer
  Given a user has the web viewer open
  And the packet has multiple workstreams
  When they click on a workstream in the sidebar
  Then the main view should display the selected workstream
  And the URL should update with the workstream ID
```

**Scenario 13.3: User searches for events in web viewer**

```gherkin
Scenario: User searches for events in web viewer
  Given a user has the web viewer open
  And the packet has many events
  When they type a search query in the search box
  Then matching events should be highlighted
  And the list should filter to show only matching events
```

**Scenario 13.4: User filters by source in web viewer**

```gherkin
Scenario: User filters by source in web viewer
  Given a user has the web viewer open
  And the packet has events from multiple sources
  When they select a source filter (e.g., "GitHub only")
  Then only events from the selected source should be displayed
```

#### Edge Cases and Error Scenarios

**Scenario 13.5: Web viewer with no packet**

```gherkin
Scenario: Web viewer with no packet
  Given a user has no rendered packet
  When they run "shiplog web-serve"
  Then an error message should indicate no packet found
  And the server should not start
```

**Scenario 13.6: Port already in use**

```gherkin
Scenario: Port already in use
  Given a user has another service using the default port
  When they run "shiplog web-serve"
  Then an error should indicate the port is in use
  And the user should be able to specify an alternative port
```

#### Integration Scenarios

**Scenario 13.7: Web viewer updates on packet re-render**

```gherkin
Scenario: Web viewer updates on packet re-render
  Given a user has the web viewer open
  And they re-render the packet with new data
  When they refresh the browser
  Then the web viewer should display the updated packet
```

#### Performance/Load Scenarios

**Scenario 13.8: Web viewer with large packet**

```gherkin
Scenario: Web viewer with large packet
  Given a user has a packet with 1,000 events
  When they open the web viewer
  Then the page should load within reasonable time (< 3 seconds)
  And scrolling should remain smooth
```

---

### 14. Plugin System

#### Primary User Scenarios

**Scenario 14.1: User installs a third-party ingest adapter plugin**

```gherkin
Scenario: User installs a third-party ingest adapter plugin
  Given a third-party has published a shiplog ingest adapter plugin
  When the user runs "shiplog plugin install shiplog-ingest-custom"
  Then the plugin should be downloaded and installed
  And the plugin should be available for use
```

**Scenario 14.2: User uses a plugin ingest adapter**

```gherkin
Scenario: User uses a plugin ingest adapter
  Given a user has installed a custom ingest adapter plugin
  And the plugin is named "custom"
  When they run "shiplog collect --source custom --config custom-config.yaml"
  Then the plugin's ingestor should be invoked
  And events should be collected using the plugin
  And events should have SourceSystem::Other("custom")
```

**Scenario 14.3: User lists installed plugins**

```gherkin
Scenario: User lists installed plugins
  Given a user has installed multiple plugins
  When they run "shiplog plugin list"
  Then all installed plugins should be listed
  And each plugin should show its name and version
```

**Scenario 14.4: User removes a plugin**

```gherkin
Scenario: User removes a plugin
  Given a user has installed a plugin
  When they run "shiplog plugin remove shiplog-ingest-custom"
  Then the plugin should be uninstalled
  And the plugin should no longer be available
```

#### Edge Cases and Error Scenarios

**Scenario 14.5: Plugin installation fails**

```gherkin
Scenario: Plugin installation fails
  Given a user attempts to install a plugin
  And the plugin download fails
  When they run "shiplog plugin install invalid-plugin"
  Then an error message should indicate the installation failed
  And the reason should be clearly stated
```

**Scenario 14.6: Plugin has incompatible version**

```gherkin
Scenario: Plugin has incompatible version
  Given a user attempts to install a plugin
  And the plugin requires a newer version of shiplog
  When they run "shiplog plugin install incompatible-plugin"
  Then an error should indicate the version incompatibility
  And the required shiplog version should be shown
```

**Scenario 14.7: Plugin crashes during execution**

```gherkin
Scenario: Plugin crashes during execution
  Given a user is using a plugin ingest adapter
  And the plugin crashes during execution
  When they run "shiplog collect --source custom"
  Then the crash should be caught
  And an error message should indicate the plugin failed
  And shiplog should not crash
```

#### Integration Scenarios

**Scenario 14.8: Plugin events merge with built-in sources**

```gherkin
Scenario: Plugin events merge with built-in sources
  Given a user has collected events from GitHub
  And they also collect events from a plugin
  Then events from both sources should be merged
  And workstreams can include events from both
```

#### Performance/Load Scenarios

**Scenario 14.9: Plugin with slow ingest**

```gherkin
Scenario: Plugin with slow ingest
  Given a user has a plugin that takes time to ingest events
  When they run "shiplog collect --source custom"
  Then progress should be displayed
  And the user should be able to cancel with Ctrl+C
```

---

## Reusable Step Library

This section defines reusable Given/When/Then steps that can be composed across scenarios.

### Common Given Steps

```rust
// User setup
given("a user with GitHub token configured", |ctx| {
    ctx.strings.insert("github_token".to_string(), "github_test_token".to_string());
    ctx.strings.insert("username".to_string(), "testuser".to_string());
})

given("a user with a local git repository", |ctx| {
    ctx.paths.insert("repo_path".to_string(), "/tmp/test-repo".into());
    ctx.flags.insert("repo_exists".to_string(), true);
})

given("a user has collected events", |ctx| {
    ctx.numbers.insert("event_count".to_string(), 50);
    ctx.flags.insert("events_collected".to_string(), true);
})

given("a user has generated workstreams", |ctx| {
    ctx.numbers.insert("workstream_count".to_string(), 5);
    ctx.flags.insert("workstreams_generated".to_string(), true);
})

given("a user has a shiplog config file", |ctx| {
    ctx.paths.insert("config_path".to_string(), "/tmp/shiplog.yaml".into());
    ctx.flags.insert("config_exists".to_string(), true);
})

given("the cache is enabled", |ctx| {
    ctx.flags.insert("cache_enabled".to_string(), true);
    ctx.paths.insert("cache_path".to_string(), "/tmp/cache.db".into());
})

// Date ranges
given("a date range for the past quarter", |ctx| {
    ctx.strings.insert("since".to_string(), "2025-01-01".to_string());
    ctx.strings.insert("until".to_string(), "2025-04-01".to_string());
})

given("a date range for the past month", |ctx| {
    ctx.strings.insert("since".to_string(), "2025-03-01".to_string());
    ctx.strings.insert("until".to_string(), "2025-04-01".to_string());
})

// Repository setup
given("a GitHub repository with recent PRs", |ctx| {
    ctx.strings.insert("repo".to_string(), "owner/repo".to_string());
    ctx.numbers.insert("pr_count".to_string(), 25);
})

given("a GitLab repository with recent MRs", |ctx| {
    ctx.strings.insert("gitlab_instance".to_string(), "gitlab.com".to_string());
    ctx.strings.insert("gitlab_repo".to_string(), "owner/repo".to_string());
    ctx.numbers.insert("mr_count".to_string(), 15);
})

// LLM setup
given("an LLM API key is configured", |ctx| {
    ctx.strings.insert("llm_api_key".to_string(), "sk_test_key".to_string());
    ctx.strings.insert("llm_model".to_string(), "gpt-4".to_string());
})
```

### Common When Steps

```rust
// Collection
when("they collect events from GitHub", |ctx| {
    ctx.flags.insert("api_call_made".to_string(), true);
    ctx.numbers.insert("collected_events".to_string(), 25);
    Ok(())
})

when("they collect events from local git", |ctx| {
    ctx.flags.insert("git_read".to_string(), true);
    ctx.numbers.insert("collected_events".to_string(), 30);
    Ok(())
})

when("they collect events from GitLab", |ctx| {
    ctx.flags.insert("gitlab_api_call_made".to_string(), true);
    ctx.numbers.insert("collected_events".to_string(), 15);
    Ok(())
})

when("they collect events from Jira", |ctx| {
    ctx.flags.insert("jira_api_call_made".to_string(), true);
    ctx.numbers.insert("collected_events".to_string(), 10);
    Ok(())
})

// Processing
when("they generate workstreams", |ctx| {
    ctx.flags.insert("workstreams_generated".to_string(), true);
    ctx.numbers.insert("workstream_count".to_string(), 5);
    Ok(())
})

when("they cluster events using LLM", |ctx| {
    ctx.flags.insert("llm_called".to_string(), true);
    ctx.numbers.insert("workstream_count".to_string(), 5);
    Ok(())
})

when("they merge events from multiple sources", |ctx| {
    ctx.flags.insert("merged".to_string(), true);
    ctx.numbers.insert("total_events".to_string(), 80);
    Ok(())
})

// Rendering
when("they render the packet", |ctx| {
    ctx.flags.insert("packet_rendered".to_string(), true);
    ctx.paths.insert("packet_path".to_string(), "/out/run_001/packet.md".into());
    Ok(())
})

when("they render with a custom template", |ctx| {
    ctx.flags.insert("packet_rendered".to_string(), true);
    ctx.flags.insert("custom_template_used".to_string(), true);
    ctx.paths.insert("packet_path".to_string(), "/out/run_001/packet.md".into());
    Ok(())
})

// Cache operations
when("they check cache stats", |ctx| {
    ctx.numbers.insert("cache_total".to_string(), 100);
    ctx.numbers.insert("cache_valid".to_string(), 80);
    ctx.numbers.insert("cache_expired".to_string(), 20);
    Ok(())
})

when("they clear the cache", |ctx| {
    ctx.numbers.insert("cache_total".to_string(), 0);
    ctx.flags.insert("cache_cleared".to_string(), true);
    Ok(())
})
```

### Common Then Steps

```rust
// Event verification
then("events should be collected", |ctx| {
    let count = ctx.number("collected_events").unwrap_or(0);
    assert_true(count > 0, "event count > 0")
})

then("events should have correct source", |ctx| {
    let source = ctx.string("source").unwrap_or("unknown");
    assert_true(source != "unknown", "source is set")
})

then("no API calls should be made", |ctx| {
    assert_false(ctx.flag("api_call_made").unwrap_or(true), "API call made")
})

// Workstream verification
then("workstreams should be generated", |ctx| {
    assert_true(ctx.flag("workstreams_generated").unwrap_or(false), "workstreams generated")
})

then("workstreams should contain events from all sources", |ctx| {
    assert_true(ctx.flag("multi_source_events").unwrap_or(false), "multi-source events")
})

// Packet verification
then("the packet should exist", |ctx| {
    assert_true(ctx.flag("packet_rendered").unwrap_or(false), "packet rendered")
})

then("the packet should include all workstreams", |ctx| {
    let ws_count = ctx.number("workstream_count").unwrap_or(0);
    assert_true(ws_count > 0, "workstream count > 0")
})

// Cache verification
then("cache should be used", |ctx| {
    assert_true(ctx.flag("cache_hit").unwrap_or(false), "cache hit")
})

then("cache should be cleared", |ctx| {
    assert_true(ctx.flag("cache_cleared").unwrap_or(false), "cache cleared")
    let total = ctx.number("cache_total").unwrap_or(0);
    assert_eq(total, 0, "cache total")
})

// Error verification
then("the command should fail with an error", |ctx| {
    assert_true(ctx.flag("command_failed").unwrap_or(false), "command failed")
})

then("an error message should be displayed", |ctx| {
    let error = ctx.string("error_message").unwrap_or("");
    assert_true(!error.is_empty(), "error message not empty")
})
```

---

## Implementation Guidelines

### Test Organization

```
crates/shiplog-testkit/src/
├── bdd.rs                    # Core BDD framework (existing)
├── bdd_tests.rs              # Existing workflow tests
├── scenarios/
│   ├── v02x/
│   │   ├── binary_releases.rs
│   │   ├── local_git_ingest.rs
│   │   ├── packet_formatting.rs
│   │   └── cache_improvements.rs
│   ├── v03x/
│   │   ├── gitlab_ingest.rs
│   │   ├── jira_linear_ingest.rs
│   │   ├── multi_source_merging.rs
│   │   ├── configurable_templates.rs
│   │   └── llm_clustering.rs
│   └── later/
│       ├── team_aggregation.rs
│       ├── cron_mode.rs
│       ├── tui_editor.rs
│       ├── web_viewer.rs
│       └── plugin_system.rs
└── steps/
    ├── common_steps.rs       # Reusable steps
    ├── ingest_steps.rs       # Ingest-specific steps
    ├── cluster_steps.rs      # Clustering-specific steps
    ├── render_steps.rs       # Rendering-specific steps
    └── cache_steps.rs        # Cache-specific steps
```

### Scenario Naming Convention

Use descriptive, user-centric names:

- ✅ "User downloads and installs binary for their platform"
- ✅ "User ingests commits from local git repository"
- ❌ "Binary download works"
- ❌ "Git ingest function"

### Step Description Guidelines

**Given steps** should describe preconditions:
- ✅ "a user with GitHub token configured"
- ✅ "a date range for the past quarter"
- ❌ "setup user"
- ❌ "configure dates"

**When steps** should describe user actions:
- ✅ "they collect events from GitHub"
- ✅ "they render the packet"
- ❌ "collect events"
- ❌ "render"

**Then steps** should describe expected outcomes:
- ✅ "events should be collected"
- ✅ "the packet should exist"
- ❌ "events collected"
- ❌ "packet exists"

### Context Key Naming

Use consistent, descriptive keys:

| Key Type | Pattern | Examples |
|----------|---------|----------|
| Strings | `snake_case` | `username`, `repo`, `since` |
| Numbers | `snake_case` | `event_count`, `pr_count`, `cache_total` |
| Flags | `snake_case` | `events_collected`, `cache_hit`, `command_failed` |
| Paths | `snake_case` | `repo_path`, `config_path`, `packet_path` |

### Assertion Usage

Prefer specific assertions over generic ones:

```rust
// Good
assert_eq(count, 25, "event count")?
assert_true(ctx.flag("cache_hit").unwrap_or(false), "cache hit")?

// Avoid
assert_true(true, "something happened")?
```

### Test Data Management

Use fixture builders for consistent test data:

```rust
use crate::bdd::builders::*;

let event = EventBuilder::new("owner/repo", 42, "Fix bug")
    .kind(EventKind::PullRequest)
    .build();

let coverage = CoverageBuilder::new("alice")
    .dates(since, until)
    .completeness(Completeness::Complete)
    .build();
```

### Mock External Dependencies

For scenarios involving external APIs (GitHub, GitLab, Jira, LLM), use mock backends:

```rust
use shiplog_cluster_llm::MockLlmBackend;

let mock_backend = MockLlmBackend::new();
mock_backend.set_response("workstream: [1,2,3]");
let clusterer = LlmClusterer::new(Box::new(mock_backend), config);
```

### Performance Test Guidelines

For performance scenarios:

1. Set reasonable thresholds based on expected usage
2. Measure both time and memory
3. Use consistent test data sizes
4. Document the hardware/environment assumptions

Example thresholds:
- Binary startup: < 100ms
- Cache stats: < 1 second
- Render 500 receipts: < 5 seconds
- Collect 2,000 MRs: < 60 seconds
- Cluster 1,000 events: < 60 seconds

---

## Appendix: Scenario Cross-Reference

| Feature | Primary Scenarios | Edge Cases | Integration | Performance |
|---------|------------------|------------|-------------|-------------|
| Binary Releases | 1.1-1.3 | 1.4-1.6 | 1.7 | 1.8 |
| Local Git Ingest | 2.1-2.3 | 2.4-2.8 | 2.9-2.11 | 2.12-2.13 |
| Packet Formatting | 3.1-3.4 | 3.5-3.7 | 3.8-3.9 | 3.10 |
| Cache Improvements | 4.1-4.5 | 4.6-4.8 | 4.9-4.10 | 4.11-4.12 |
| GitLab Ingest | 5.1-5.4 | 5.5-5.8 | 5.9-5.11 | 5.12 |
| Jira/Linear Ingest | 6.1-6.4 | 6.5-6.8 | 6.9-6.10 | 6.11 |
| Multi-Source Merging | 7.1-7.3 | 7.4-7.6 | 7.7-7.8 | 7.9 |
| Configurable Templates | 8.1-8.4 | 8.5-8.7 | 8.8-8.9 | 8.10 |
| LLM Clustering | 9.1-9.3 | 9.4-9.6 | 9.7-9.8 | 9.9 |
| Team Aggregation | 10.1-10.3 | 10.4-10.5 | 10.6 | 10.7 |
| Cron Mode | 11.1-11.3 | 11.4-11.5 | 11.6 | 11.7 |
| TUI Editor | 12.1-12.6 | 12.7-12.8 | 12.9 | 12.10 |
| Web Viewer | 13.1-13.4 | 13.5-13.6 | 13.7 | 13.8 |
| Plugin System | 14.1-14.4 | 14.5-14.7 | 14.8 | 14.9 |

---

## Summary

This design document provides comprehensive BDD scenarios for all shiplog ROADMAP features across three horizons:

- **v0.2.x (Now)**: 4 features, 36 scenarios
- **v0.3.x (Next)**: 5 features, 39 scenarios  
- **Later (Exploratory)**: 5 features, 33 scenarios

**Total: 108 scenarios** covering:
- Primary user workflows
- Edge cases and error handling
- Integration between features
- Performance and load testing

Each scenario is designed to be:
- **User-centric**: Describes behavior from the user's perspective
- **Clear**: Uses descriptive Given/When/Then steps
- **Reusable**: Steps can be composed across scenarios
- **Testable**: Can be implemented using the existing BDD framework

The reusable step library provides a foundation for efficient test implementation, and the implementation guidelines ensure consistency across the test suite.
