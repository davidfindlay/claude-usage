use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local, Utc};
use colored::Colorize;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::process::Command;

// ─── API Response Types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct UsageWindow {
    utilization: f64,        // 0–100 percent
    resets_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    five_hour: Option<UsageWindow>,
    seven_day: Option<UsageWindow>,
    seven_day_opus: Option<UsageWindow>,
}

// ─── Keychain credential reading (macOS) ─────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OAuthCredentials {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: OAuthToken,
}

#[derive(Debug, Deserialize)]
struct OAuthToken {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "subscriptionType")]
    subscription_type: Option<String>,
}

fn get_token_from_keychain() -> Result<OAuthToken> {
    let output = Command::new("security")
        .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
        .output()
        .context("Failed to run 'security' command — are you on macOS?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Could not read Claude Code credentials from Keychain.\n\
             Make sure Claude Code is installed and you've logged in.\n\
             Error: {stderr}"
        );
    }

    let raw = String::from_utf8(output.stdout)
        .context("Keychain output was not valid UTF-8")?;
    let raw = raw.trim();

    let creds: OAuthCredentials = serde_json::from_str(raw)
        .context("Could not parse Claude Code credentials from Keychain")?;

    Ok(creds.claude_ai_oauth)
}

// ─── API call ─────────────────────────────────────────────────────────────────

fn fetch_usage(token: &str) -> Result<UsageResponse> {
    let client = Client::new();
    let resp = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "claude-code/2.0.32")
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Accept", "application/json")
        .send()
        .context("Failed to reach Anthropic API")?;

    let status = resp.status();
    if status == 401 {
        bail!("Token expired or invalid — try logging out and back in with Claude Code:\n  claude logout && claude");
    }
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        bail!("API returned {status}: {body}");
    }

    resp.json::<UsageResponse>().context("Failed to parse usage response")
}

// ─── Display ─────────────────────────────────────────────────────────────────

fn usage_bar(pct: f64, width: usize) -> colored::ColoredString {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

    // Colour by how full it is
    if pct >= 90.0 {
        bar.red().bold()
    } else if pct >= 70.0 {
        bar.yellow()
    } else {
        bar.green()
    }
}

fn format_reset(resets_at: &Option<String>) -> String {
    let Some(ts) = resets_at else {
        return "—".dimmed().to_string();
    };
    // Parse ISO 8601 and convert to local time
    let Ok(dt) = ts.parse::<DateTime<Utc>>() else {
        return ts.clone();
    };
    let local: DateTime<Local> = dt.into();
    let now = Local::now();
    let diff = dt.signed_duration_since(Utc::now());

    let mins = diff.num_minutes();
    let hours = diff.num_hours();

    let relative = if diff.num_seconds() <= 0 {
        "now".green().to_string()
    } else if mins < 60 {
        format!("in {}m", mins).yellow().to_string()
    } else if hours < 24 {
        format!("in {}h {}m", hours, mins - hours * 60).normal().to_string()
    } else {
        format!("in {}d", diff.num_days()).normal().to_string()
    };

    format!(
        "{} ({})",
        local.format("%H:%M").to_string().dimmed(),
        relative
    )
}

fn print_window(label: &str, window: &Option<UsageWindow>, bar_width: usize) {
    match window {
        None => {
            println!("  {:<18} {}", label, "not available".dimmed());
        }
        Some(w) => {
            let pct = w.utilization.min(100.0);
            let bar = usage_bar(pct, bar_width);
            let pct_str = if pct >= 90.0 {
                format!("{:5.1}%", pct).red().bold()
            } else if pct >= 70.0 {
                format!("{:5.1}%", pct).yellow()
            } else {
                format!("{:5.1}%", pct).green()
            };

            println!(
                "  {:<18} {} {} resets {}",
                label.bold(),
                bar,
                pct_str,
                format_reset(&w.resets_at)
            );
        }
    }
}

fn print_plain(label: &str, window: &Option<UsageWindow>) {
    match window {
        None => println!("{}: N/A", label),
        Some(w) => {
            let pct = w.utilization.min(100.0);
            let resets = w
                .resets_at
                .as_deref()
                .unwrap_or("—");
            println!("{}: {:.1}% Resets: {}", label, pct, resets);
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("\n  {} {}\n", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let plain = std::env::args().any(|a| a == "--plain" || a == "-p");

    if !plain {
        println!();
        print!("  {} Fetching usage data... ", "◆".cyan());
    }

    let token = get_token_from_keychain()?;
    let usage = fetch_usage(&token.access_token)?;

    if plain {
        print_plain("5hr session", &usage.five_hour);
        print_plain("7 day rolling", &usage.seven_day);
        return Ok(());
    }

    // Clear the "fetching" line
    print!("\r{}\r", " ".repeat(50));

    // Header
    let plan = token
        .subscription_type
        .as_deref()
        .unwrap_or("unknown")
        .to_uppercase();

    println!(
        "  {} Claude {} Plan — Usage Limits",
        "◆".cyan().bold(),
        plan.yellow().bold()
    );
    println!("  {}", "─".repeat(65).dimmed());

    let bar_width = 28;
    print_window("5-hour session", &usage.five_hour, bar_width);
    print_window("7-day rolling", &usage.seven_day, bar_width);

    // Only show Opus row if it has data (Max plan)
    if let Some(ref opus) = usage.seven_day_opus {
        if opus.utilization > 0.0 || opus.resets_at.is_some() {
            print_window("7-day (Opus)", &usage.seven_day_opus, bar_width);
        }
    }

    println!("  {}", "─".repeat(65).dimmed());

    // Summary hint
    let highest = [&usage.five_hour, &usage.seven_day]
        .iter()
        .filter_map(|w| w.as_ref().map(|w| w.utilization))
        .fold(0.0_f64, f64::max);

    if highest >= 90.0 {
        println!(
            "\n  {} You're nearly at your limit — check your reset time above.",
            "⚠".red().bold()
        );
    } else if highest >= 70.0 {
        println!(
            "\n  {} Usage is elevated — consider pacing your next session.",
            "△".yellow()
        );
    } else {
        println!(
            "\n  {} Looking good — plenty of capacity remaining.",
            "✓".green()
        );
    }

    println!();
    Ok(())
}
