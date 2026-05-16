use anyhow::Result;
use chrono::{Datelike, Months, NaiveDate, Utc};
use clap::Args;

/// CLI flags that resolve to an inclusive/exclusive date window.
#[derive(Args, Debug, Clone, Default)]
pub(crate) struct DateArgs {
    /// Start date (inclusive), YYYY-MM-DD.
    #[arg(long)]
    pub(crate) since: Option<NaiveDate>,
    /// End date (exclusive), YYYY-MM-DD.
    #[arg(long)]
    pub(crate) until: Option<NaiveDate>,
    /// Use the last six months, ending today.
    #[arg(long)]
    pub(crate) last_6_months: bool,
    /// Use the previous calendar quarter.
    #[arg(long)]
    pub(crate) last_quarter: bool,
    /// Use a calendar year.
    #[arg(long)]
    pub(crate) year: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedWindow {
    pub(crate) since: NaiveDate,
    pub(crate) until: NaiveDate,
    pub(crate) label: WindowLabel,
    pub(crate) period: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowLabel {
    Explicit,
    LastSixMonths,
    LastQuarter,
    Year(i32),
}

impl ResolvedWindow {
    pub(crate) fn window_label(&self) -> String {
        let label = match self.label {
            WindowLabel::Explicit => format!("{}..{}", self.since, self.until),
            WindowLabel::LastSixMonths => {
                format!("last-6-months ({}..{})", self.since, self.until)
            }
            WindowLabel::LastQuarter => {
                format!("last-quarter ({}..{})", self.since, self.until)
            }
            WindowLabel::Year(year) => format!("{year} ({}..{})", self.since, self.until),
        };
        if let Some(period) = &self.period {
            format!("{period} ({}..{})", self.since, self.until)
        } else {
            label
        }
    }

    pub(crate) fn with_period(mut self, period: impl Into<String>) -> Self {
        self.period = Some(period.into());
        self
    }
}

pub(crate) fn resolve_date_window(args: DateArgs) -> Result<ResolvedWindow> {
    resolve_date_window_for_today(args, Utc::now().date_naive())
}

pub(crate) fn resolve_date_window_for_today(
    args: DateArgs,
    today: NaiveDate,
) -> Result<ResolvedWindow> {
    match (args.since, args.until) {
        (Some(since), Some(until)) => return checked_window(since, until, WindowLabel::Explicit),
        (Some(_), None) | (None, Some(_)) => {
            anyhow::bail!("provide both --since and --until, or use a date preset")
        }
        (None, None) => {}
    }

    let preset_count = usize::from(args.last_6_months)
        + usize::from(args.last_quarter)
        + usize::from(args.year.is_some());
    if preset_count > 1 {
        anyhow::bail!("choose only one date preset: --last-6-months, --last-quarter, or --year")
    }

    if let Some(year) = args.year {
        let since = NaiveDate::from_ymd_opt(year, 1, 1)
            .ok_or_else(|| anyhow::anyhow!("invalid --year value: {year}"))?;
        let until = NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .ok_or_else(|| anyhow::anyhow!("invalid --year value: {year}"))?;
        return checked_window(since, until, WindowLabel::Year(year));
    }

    if args.last_quarter {
        let start_of_current_quarter = quarter_start(today.year(), today.month())?;
        let previous_quarter_anchor = start_of_current_quarter
            .checked_sub_months(Months::new(3))
            .ok_or_else(|| anyhow::anyhow!("could not resolve --last-quarter"))?;
        return checked_window(
            previous_quarter_anchor,
            start_of_current_quarter,
            WindowLabel::LastQuarter,
        );
    }

    let since = today
        .checked_sub_months(Months::new(6))
        .ok_or_else(|| anyhow::anyhow!("could not resolve --last-6-months"))?;
    checked_window(since, today, WindowLabel::LastSixMonths)
}

pub(crate) fn checked_window(
    since: NaiveDate,
    until: NaiveDate,
    label: WindowLabel,
) -> Result<ResolvedWindow> {
    if since >= until {
        anyhow::bail!("date window must satisfy --since < --until")
    }
    Ok(ResolvedWindow {
        since,
        until,
        label,
        period: None,
    })
}

fn quarter_start(year: i32, month: u32) -> Result<NaiveDate> {
    let start_month = match month {
        1..=3 => 1,
        4..=6 => 4,
        7..=9 => 7,
        10..=12 => 10,
        _ => anyhow::bail!("invalid month while resolving quarter: {month}"),
    };
    NaiveDate::from_ymd_opt(year, start_month, 1)
        .ok_or_else(|| anyhow::anyhow!("invalid quarter start for {year}-{start_month:02}"))
}
