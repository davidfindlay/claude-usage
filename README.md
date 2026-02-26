# claude-usage

A tiny Rust CLI that shows your **Claude Pro/Max plan usage limits** — exactly what `claude /usage` shows inside Claude Code, but available anywhere in your terminal.

## What it shows

```
  ◆ Claude PRO Plan — Usage Limits
  ─────────────────────────────────────────────────────────────────
  5-hour session     ████████████░░░░░░░░░░░░░░░░  42.0% resets 14:23 (in 2h 15m)
  7-day rolling      ████░░░░░░░░░░░░░░░░░░░░░░░░  15.3% resets Thu 09:00 (in 3d)
  ─────────────────────────────────────────────────────────────────

  ✓ Looking good — plenty of capacity remaining.
```

Bars turn **yellow** above 70% and **red** above 90%.

## How it works

Claude Code stores your OAuth session token in macOS Keychain under `"Claude Code-credentials"`. This tool reads that token and calls the same endpoint Claude Code itself uses: `https://api.anthropic.com/api/oauth/usage`.

**Requires:** macOS + Claude Code installed and logged in.

## Install

```bash
# You need Rust: https://rustup.rs
cargo install --path .

# Then just run:
claude-usage
```

Or run without installing:
```bash
cargo run
```

## License

GPL-3.0 (see `LICENSE`).

## Tips

- Run it before a heavy Claude Code session to check your headroom
- The 5-hour window resets 5 hours after your *first* message, not on a fixed clock
- Usage is shared across claude.ai, Claude Code, and the Claude desktop app
- If the token is expired, just run `claude` in your terminal to refresh it

## Troubleshooting

- **Could not read credentials**
  - Ensure Claude Code is logged in (`claude`), or set `CLAUDE_CODE_OAUTH_TOKEN`.
- **401 from API**
  - Re-authenticate: `claude logout && claude`
- **No Keychain (Linux)**
  - Ensure `~/.claude/.credentials.json` exists, or provide env token.

## Privacy & security

- This tool reads local OAuth credentials to query usage.
- It does **not** print raw tokens.
- Avoid sharing output publicly if your usage details are sensitive.
