@AGENTS.md
@docs/AGENT_WORKFLOW.md

# Claude Code

Rules above apply unchanged. This section only maps the agent team to Claude Code.

## Subagents

Role definitions live in `.claude/agents/`. The main session coordinates: it plans, waits for user approval, delegates with an explicit file scope, and verifies results before reporting.

| Role | Subagent | Model | Effort | Edits files |
|---|---|---|---|---|
| Main agent (session) | — | `opus` | `high` | Yes |
| Rust Developer | `rust-developer` | `opus` | `medium` | Assigned Rust files |
| Researcher | `researcher` | `opus` | `high` | No |
| Terminal UI | `terminal-ui` | `sonnet` | `medium` | Assigned UI files |
| Tester | `tester` | `sonnet` | `medium` | Assigned test files |
| Reviewer | `reviewer` | `sonnet` | `medium` | No |
| User Advocate | `user-advocate` | `sonnet` | `medium` | No |

- The session model and effort come from `.claude/settings.json` (`model`, `effortLevel`); `/model`, `--model` or `ANTHROPIC_MODEL` override them.
- Subagent model and effort come from each file's `model` and `effort` frontmatter. The `CLAUDE_CODE_EFFORT_LEVEL` environment variable overrides `effort`.
- Give every delegation: goal, files it may edit, files it must not touch, checks to run and the handoff format. Never let two subagents edit the same file at once.
- Subagents never push, tag or start work from a later milestone. Only the main session commits, after checks pass.
