# Workflow — multi-agent feature pipeline

Six-agent pipeline: feature spec in, reviewed commits out. Manager breaks spec into
atomic tasks (1 task = 1 commit, diff ~100–150 lines). Each task fans out to
impl/test/docs specialists, gates through Reviewer, lands via Git Agent.

## Agents (in pipeline order)

| # | File | Role | Input | Output |
|---|------|------|-------|--------|
| 1 | [manager.md](manager.md) | Manager — breaks `feature_plan.md` into atomic tasks. Never writes code. | `feature_plan.md` | `task_breakdown.md`, `commit_plan.md` |
| 2 | [impl-specialist.md](impl-specialist.md) | Impl Specialist — implements one task's `files_to_touch`. No tests, no docs. | `task_breakdown.md` (task block) | `patches/<task_id>.patch` |
| 3 | [test-specialist.md](test-specialist.md) | Test Specialist — writes tests covering `acceptance_criteria`. No production code. | `patches/<task_id>.patch` | `patches/<task_id>-tests.patch` |
| 4 | [docs-specialist.md](docs-specialist.md) | Docs Specialist — updates doc comments/README/CHANGELOG for the diff. No code/tests. | `patches/<task_id>*.patch` | `patches/<task_id>-docs.patch` |
| 5 | [reviewer.md](reviewer.md) | Reviewer — gate before commit: diff size, criteria coverage, clippy/valgrind, plan consistency. Never edits. | all patches for a `task_id` | `review_report.md` |
| 6 | [git-agent.md](git-agent.md) | Git Agent — applies approved patches, commits (impl/test/docs as separate commits). Never decides what to commit. | `commit_plan.md` + `review_report.md` (`approved`) | commits on repo |

## How to run it

1. Write the feature spec as `feature_plan.md`.
2. Dispatch **manager.md** on it → get `task_breakdown.md` + `commit_plan.md`.
3. For each `task_id` in `commit_plan.md` order:
   - dispatch **impl-specialist.md** → `patches/<task_id>.patch`
   - dispatch **test-specialist.md** → `patches/<task_id>-tests.patch`
   - dispatch **docs-specialist.md** → `patches/<task_id>-docs.patch`
   - dispatch **reviewer.md** → `review_report.md` (must be `approved` to proceed)
   - dispatch **git-agent.md** → applies + commits (impl, test, docs as separate commits)
4. Repeat until every `task_id` in `commit_plan.md` is committed.

## Hard rules across the pipeline

- 1 task_id → up to 3 commits (`impl`, `test`, `docs`), never squashed.
- impl diff ≤ 150 lines (specialist self-limit) / ≤ 200 lines (reviewer gate) / ≤ 400 lines in the
  `.patch` file (git-agent hard abort → `size_violation`).
- Refactor and new feature are always separate tasks/commits.
- `unsafe` Rust requires `// SAFETY:` on every block — enforced by impl-specialist and reviewer.
- Commit format: `<type>(<scope>): <description>` + `Why:` + `Task: <task_id>` body — see
  [git-agent.md](git-agent.md) for accepted types.
- `git add .` and squashing distinct tasks are forbidden for the Git Agent.

## On-demand quality guidance

All agents use [`../docs/code-quality/README.md`](../docs/code-quality/README.md) to select quality
references. They must not load the entire directory. The Manager selects design guidance when a task
changes architecture; specialists select only their language, performance, testing, or security
guide; the Reviewer starts with `review-checklist.md` and follows its routing triggers.

`graph.html` in this folder is a separate artifact (not an agent spec) — leave as-is unless asked.
