# Agent CLI configuration

This repository is worked on with several agent CLIs. One file holds the rules
for all of them: [agent-rules.md](agent-rules.md), which the root `AGENTS.md`
links to. The files below adapt each CLI to those rules. They restrict or
guide; none of them lets a CLI run more without asking.

Every entry was checked against the CLI's documentation or source on
2026-11-14. No CLI other than Claude Code was run here, so the files for Codex,
the Antigravity CLI and Copilot are untested. Each CLI loads repository files
only after the user trusts the folder.

## Files

| CLI | File | What it does |
| --- | --- | --- |
| All | `AGENTS.md` | The rules. Codex, the Antigravity CLI, Copilot CLI and Claude Code read it |
| Claude Code | `CLAUDE.md` | Imports the rules file, for versions and sessions that do not read `AGENTS.md` |
| Claude Code | `.claude/settings.json` | No attribution lines. Asks before a slow lane, a force-push, a push to `main` or a workflow edit. Refuses `cargo publish` |
| Claude Code, Copilot CLI | `.mcp.json` | Registers the `hashline` server for hash-anchored edits ([TOOLS.md](../../TOOLS.md#hashline)) |
| Claude Code | `.claude/agents/mechanical-edit.md` | A subagent on the smallest model for mechanical file edits |
| Claude Code | `.claude/rules/unsafe-boundary.md` | Loads the unsafe rule when a file beside `buf.rs` is opened |
| Claude Code | `.claude/skills/make-gates/SKILL.md` | `/make-gates`: run the gates for the change |
| Codex | `.codex/rules/frost.rules` | Asks before a slow lane or a push. Refuses `cargo publish` |
| Codex | `.codex/agents/mechanical-edit.toml` | An agent role with low reasoning effort for mechanical file edits |
| Codex | `.codex/config.toml` | Registers the `hashline` server |
| Antigravity CLI | `.agents/rules/repository.md` | Repeats the critical rules as an always-on rule |
| Antigravity CLI | `.agents/agents/mechanical-edit/agent.md` | A subagent on the fast model for mechanical file edits |
| Antigravity CLI | `.agents/skills/make-gates/SKILL.md` | `/make-gates`: run the gates for the change |
| Antigravity CLI | `.agents/mcp_config.json` | Registers the `hashline` server |
| Copilot | `.github/copilot-instructions.md` | Points to the rules and says what a review checks first |
| Copilot | `.github/copilot/settings.json` | Copilot CLI adds no `Co-authored-by` trailer |
| Copilot | `.github/instructions/*.instructions.md` | Rules that load for Rust sources and for workflow files |
| Copilot | `.github/agents/mechanical-edit.agent.md` | A custom agent limited to reading, searching and editing |

The four `mechanical-edit` files and the two `make-gates` files say the same
thing in each CLI's format. Change them together.

## What a repository file cannot do

| Item | Where it is set |
| --- | --- |
| Codex `Co-authored-by: Codex` trailer | The user's Codex account settings |
| Codex profiles, model provider and notifications | The user's `~/.codex/config.toml`; Codex drops these keys from a project file |
| Antigravity CLI permissions and default model | The user's `~/.gemini/antigravity-cli/settings.json`; no workspace file is documented |
| Copilot cloud agent attribution and model | Not configurable in a repository file |
| Claude Code cloud sessions: installed tools, network access and the git author | The cloud environment's setup script and settings |
| A hard block on force-pushes or pushes to `main` | Branch protection on GitHub. The rules above ask or advise; a shell can evade them |

## Not installed, and why

- Allow rules that run the fast gates without asking. They widen what a CLI
  may do, so they are the maintainer's choice.
- Hooks for Codex, the Antigravity CLI and Copilot. Their input and blocking
  contracts were not verified here, and a wrong hook can block every tool call.
- `copilot-setup-steps.yml` for the Copilot cloud agent. It adds a workflow,
  and its steps were not verified.
- A session-start hook that installs ShellCheck, shfmt, Bats, cargo-nextest
  and cargo-llvm-cov in Claude Code cloud sessions. The cloud environment's
  setup script is the documented place for that.

## Hash-anchored edits

`hashline` 0.9.24 was built from source and run on this host on 2026-11-14:
`read`, a `patch`, the refusal of a stale anchor, and the server's tool list
all behaved as [TOOLS.md](../../TOOLS.md#hashline) describes. The server
entries were not exercised from inside any CLI. Whether Codex honours
`mcp_servers` in a project file is inferred from its source, which strips a
fixed list of keys from project files and does not list that one.

The binary must be installed on each machine. A Claude Code cloud session
does not have it unless the cloud environment's setup script installs it.
