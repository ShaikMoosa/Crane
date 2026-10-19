# Superpowers analysis for Ember Notes

Source examined: <https://github.com/obra/superpowers> at commit `bb92a77741419a4ab5f06e711a283343f1ada0c3` (release 7.0.0, 2026-10-09). The repository is MIT-licensed.

## What it is

Superpowers is not a UI framework or runtime dependency. It is a Codex-compatible plugin containing composable skills that shape how an agent discovers requirements, plans work, writes tests, debugs failures, reviews changes, and verifies completion.

## Architecture

- `.codex-plugin/plugin.json` declares the Codex plugin, metadata, and `skills/` entrypoint.
- `using-superpowers` bootstraps skill selection at the start of work.
- `brainstorming` turns an idea into an approved design.
- `writing-plans` converts that design into small implementation tasks.
- `test-driven-development` enforces red-green-refactor.
- `systematic-debugging` separates evidence gathering, pattern comparison, hypothesis testing, and implementation.
- `verification-before-completion` requires fresh command evidence before success claims.
- Additional skills cover worktrees, parallel/subagent execution, code-review exchange, branch finishing, skill authoring, and workflow diagnosis.

## Fit for this project

The strongest fit is process, not code reuse. Ember Notes records its design and plan under `docs/superpowers/`, keeps testable note-domain logic separate from DOM code, writes backend persistence tests before implementation where the toolchain permits, and reports verification gaps without guessing.

The project does not vendor the Superpowers repository. The official Codex plugin is the correct installation because its bootstrap and harness adaptation are part of the behavior; copying loose skill folders would omit that integration.

## Local integration status

The official curated Superpowers plugin is installed in this Codex environment (package version `6.4.2`). Its skills are exposed with qualified names such as `superpowers:using-superpowers`, `superpowers:brainstorming`, and `superpowers:verification-before-completion`. `AGENTS.md` uses those names so future sessions can invoke the installed workflows directly. The upstream repository analysis above remains pinned to the examined source commit; the installed curated package version is recorded separately because the two release channels need not use identical version numbers.

## Tradeoffs

- The workflow spends more time on specification and tests, which is worthwhile for persistence and destructive note actions.
- Full red-green proof requires the target toolchain. On a machine without Rust, backend tests can be written first but cannot honestly be claimed red or green until Rust is installed.
- Subagent and worktree workflows are valuable for larger changes but unnecessary for the initial single-developer MVP.
- Mandatory process can be disproportionate for tiny copy or token changes; those remain quick, explicit tasks.

