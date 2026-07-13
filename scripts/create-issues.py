#!/usr/bin/env python3
"""Create GitHub issues for Workmen MVP from the three implementation plans.

Generates:
  - 1 epic issue per plan (Plan 1, 2, 3)
  - 1 task issue per numbered Task in each plan (8 + 7 + 10 = 25 total)

All task issues reference their parent epic via a "Part of #N" link, and the
epic issues link back to every task. The body of each task issue preserves
the original plan task description so the issue is self-contained.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

REPO = Path("/Users/jojo/Github/project-workmen")
DOCS = REPO / "docs" / "superpowers" / "plans"
README = REPO / "docs" / "superpowers" / "plans" / "README.md"


def run(cmd: list[str], **kw) -> tuple[int, str, str]:
    p = subprocess.run(cmd, capture_output=True, text=True, **kw)
    return p.returncode, p.stdout, p.stderr


def gh(*args: str) -> str:
    rc, out, err = run(["gh", *args])
    if rc != 0:
        sys.stderr.write(f"gh {' '.join(args[:3])}... failed: {err}\n")
        sys.exit(1)
    return out.strip()


def parse_tasks(plan_path: Path) -> list[dict]:
    """Parse a plan doc into ordered task dicts.

    Each task heading looks like:  ### Task N: Title
    Body runs until the next  ### Task  or  ---  (plan-level separator).
    """
    text = plan_path.read_text()
    pattern = re.compile(r"^### Task (\d+): (.+?)\n", re.M)
    starts = [(m.group(1), m.group(2), m.start()) for m in pattern.finditer(text)]
    tasks: list[dict] = []
    for i, (num, title, start) in enumerate(starts):
        end = starts[i + 1][2] if i + 1 < len(starts) else text.find("\n---", start)
        body = text[start:end].rstrip()
        tasks.append({"num": int(num), "title": title.strip(), "body": body})
    return tasks


def plan_meta(plan_path: Path) -> dict:
    text = plan_path.read_text()
    title = text.splitlines()[0].lstrip("# ").strip()
    goal = ""
    for line in text.splitlines():
        if line.startswith("**Goal:**"):
            goal = line.replace("**Goal:**", "").strip()
            break
    return {"title": title, "goal": goal}


def issue_body_for_task(task: dict, plan: dict, epic_num: int) -> str:
    """Return issue body preserving original plan text plus metadata."""
    header = (
        f"> From plan: [{plan['title']}](docs/superpowers/plans/{plan['path'].name})  \n"
        f"> Part of #{epic_num} — {plan['title']}\n"
        f"> Plan task: Task {task['num']}\n\n"
        "---\n\n"
    )
    return header + task["body"]


def epic_body(plan: dict, task_nums: list[int], epic_issue_nums: list[int]) -> str:
    plan_rel = f"docs/superpowers/plans/{plan['path'].name}"
    lines = [
        f"## {plan['title']}",
        "",
        f"**Goal:** {plan['goal']}",
        "",
        f"**Source plan:** [{plan['path'].name}]({plan_rel})",
        "",
        "## Scope",
        "",
    ]
    for n in task_nums:
        lines.append(f"- [ ] #{epic_issue_nums[task_nums.index(n)]} — Task {n}")
    lines += [
        "",
        "## Acceptance gate",
        "",
        "All Tasks complete and the plan's **Plan Acceptance Gate** checkboxes pass.",
        "",
        "## Hard gate",
        "",
        "Do not start the next plan until this epic's acceptance gate is met.",
    ]
    return "\n".join(lines) + "\n"


def main() -> None:
    os.chdir(REPO)

    plan_files = [
        DOCS / "2026-07-13-workmen-core-validation.md",
        DOCS / "2026-07-13-workmen-desktop-workbench.md",
        DOCS / "2026-07-13-workmen-sprite-atlas-pipeline.md",
    ]
    plan_labels = [
        ["plan-1-core-validation", "phase-bootstrap"],
        ["plan-2-desktop-workbench", "phase-gui-shell"],
        ["plan-3-sprite-atlas-pipeline", "phase-pipeline"],
    ]
    # Phase labels per task (added on top of the plan label)
    task_phase_overrides = {
        # Plan 1
        (1, 1): ["phase-bootstrap"],
        (1, 2): ["phase-domain"],
        (1, 3): ["phase-domain"],
        (1, 4): ["phase-scanner"],
        (1, 5): ["phase-scanner"],
        (1, 6): ["phase-profiles", "phase-validation"],
        (1, 7): ["phase-validation"],
        (1, 8): ["phase-acceptance"],
        # Plan 2
        (2, 1): ["phase-gui-shell"],
        (2, 2): ["phase-gui-shell"],
        (2, 3): ["phase-gui-shell"],
        (2, 4): ["phase-inspector"],
        (2, 5): ["phase-profiles"],
        (2, 6): ["phase-validation"],
        (2, 7): ["phase-acceptance"],
        # Plan 3
        (3, 1): ["phase-pipeline"],
        (3, 2): ["phase-pipeline"],
        (3, 3): ["phase-sprite"],
        (3, 4): ["phase-sprite"],
        (3, 5): ["phase-atlas"],
        (3, 6): ["phase-atlas"],
        (3, 7): ["phase-atlas"],
        (3, 8): ["phase-generator"],
        (3, 9): ["phase-pipeline"],
        (3, 10): ["phase-acceptance"],
    }

    # 1) Create the three epic issues first so we can reference their numbers.
    epic_numbers: list[int] = []
    for plan_file, (plan_label, _) in zip(plan_files, plan_labels):
        meta = plan_meta(plan_file)
        meta["path"] = plan_file
        meta["label"] = plan_label
        epic_title = f"[EPIC] {meta['title']}"
        # Body is filled later once task numbers are known; for now write a
        # placeholder that includes plan goal + scope list stub.
        body = (
            f"## {meta['title']}\n\n"
            f"**Goal:** {meta['goal']}\n\n"
            f"**Source plan:** docs/superpowers/plans/{plan_file.name}\n\n"
            "_Tasks will be linked here once created._\n"
        )
        url = gh(
            "issue", "create",
            "--repo", "MikHaiLz404/project-workmen",
            "--title", epic_title,
            "--body", body,
            "--label", plan_label,
        )
        # gh prints the issue URL on stdout
        num = int(url.rstrip("/").split("/")[-1])
        epic_numbers.append(num)
        print(f"Epic  -> #{num}  {epic_title}")

    # 2) Create one issue per task, collect numbers, then patch epic bodies.
    plan_task_numbers: list[list[int]] = []
    for plan_idx, plan_file in enumerate(plan_files):
        meta = plan_meta(plan_file)
        meta["path"] = plan_file
        tasks = parse_tasks(plan_file)
        epic_num = epic_numbers[plan_idx]
        labels = plan_labels[plan_idx]
        created: list[int] = []
        for t in tasks:
            phase = task_phase_overrides.get((plan_idx + 1, t["num"]), [])
            body = issue_body_for_task(t, meta, epic_num)
            title = f"[P{plan_idx+1}.T{t['num']}] {t['title']}"
            cmd = [
                "issue", "create",
                "--repo", "MikHaiLz404/project-workmen",
                "--title", title,
                "--body", body,
            ]
            for lbl in labels + phase:
                cmd += ["--label", lbl]
            url = gh(*cmd)
            num = int(url.rstrip("/").split("/")[-1])
            created.append(num)
            print(f"  P{plan_idx+1}.T{t['num']:>2}  -> #{num}  {t['title']}")
        plan_task_numbers.append(created)

    # 3) Patch epic bodies with linked task numbers.
    for plan_idx, plan_file in enumerate(plan_files):
        meta = plan_meta(plan_file)
        meta["path"] = plan_file
        tasks = parse_tasks(plan_file)
        new_body = epic_body(
            meta,
            [t["num"] for t in tasks],
            plan_task_numbers[plan_idx],
        )
        gh(
            "issue", "edit", str(epic_numbers[plan_idx]),
            "--repo", "MikHaiLz404/project-workmen",
            "--body", new_body,
        )
        print(f"Epic #{epic_numbers[plan_idx]} body updated with task links")

    # 4) Apply milestone = one per plan
    milestone_names = [
        ("M1: Core + Scanner + Validation", plan_task_numbers[0]),
        ("M2: Desktop Workbench",          plan_task_numbers[1]),
        ("M3: Sprite + Atlas + Pipeline",  plan_task_numbers[2]),
    ]
    for ms_title, task_nums in milestone_names:
        rc, out, err = run([
            "gh", "api",
            "--method", "POST",
            "-H", "Accept: application/vnd.github+json",
            f"repos/MikHaiLz404/project-workmen/milestones",
            "-f", f"title={ms_title}",
        ])
        if rc != 0:
            print(f"milestone create failed: {err}", file=sys.stderr)
            continue
        ms = json.loads(out)
        ms_num = ms["number"]
        # assign all tasks in this plan to the milestone
        for issue_num in task_nums:
            run([
                "gh", "issue", "edit", str(issue_num),
                "--repo", "MikHaiLz404/project-workmen",
                "--milestone", ms_title,
            ])
        # assign the epic too
        run([
            "gh", "issue", "edit", str(epic_numbers[milestone_names.index((ms_title, task_nums))]),
            "--repo", "MikHaiLz404/project-workmen",
            "--milestone", ms_title,
        ])
        print(f"Milestone {ms_title} -> #{ms_num}, {len(task_nums)+1} issues assigned")

    print("\nDone.")
    print(f"  Epics: {epic_numbers}")
    print(f"  Tasks: {sum(len(p) for p in plan_task_numbers)}")


if __name__ == "__main__":
    main()
