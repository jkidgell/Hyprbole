# Hyprbole
Wallpaper changing TUI designed to work with hyprpaper

## Session Start
Run `bd prime` to recover project context (which stages are done, active work, prior decisions).

## Issue Tracking
Use beads for cross-session continuity only:
- `bd create` — before starting any non-trivial task (captures intent for future sessions)
- `bd close` — when the task is done
- `bd sync --flush-only` — at session end

**Skip** in-session status updates (`in_progress`, `blocked`, dependency graphs).
These add overhead without benefit for single-agent work — context is held in the session.

## LLM Routing
Local model server: llama.cpp on localhost:8080 — accessed via `mcp__deepseek_coder__local_llm_*` tools.
Check reachability with: `curl -s http://localhost:8080/health` (expect `{"status":"ok"}`)

**Routine tasks** (boilerplate, simple structs, basic CRUD, docstrings):
Use `local_llm_*` MCP tools first. If the local LLM times out or returns poor output,
fall back to direct generation without retrying — note the fallback with `report_routing_decision`.

**Complex tasks** (state machines, async coordination, architecture, debugging):
Implement directly. Do NOT delegate to local_llm.

Per-stage LLM strategy is documented in `PLAN.md`.

## Git
Author: jkidgell@gmail.com
