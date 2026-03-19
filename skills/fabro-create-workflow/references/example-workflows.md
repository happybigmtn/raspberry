# Fabro Workflow Topology Examples

Use these as generic Fabro patterns. For supervisory-plane lane design, also
read `raspberry-examples.md`.

## 1. Linear Prompt

Use for a one-shot plan, review, or summary.

```dot
digraph Hello {
    graph [goal="Write a short project summary"]
    rankdir=LR

    start [shape=Mdiamond, label="Start"]
    exit  [shape=Msquare, label="Exit"]

    summarize [shape=tab, label="Summarize", prompt="Summarize the repository."]

    start -> summarize -> exit
}
```

## 2. Command Then Analyze

Use when shell output is the input to a prompt step.

```dot
digraph ScanAndAnalyze {
    graph [goal="Inspect the repository and summarize it"]
    rankdir=LR

    start [shape=Mdiamond, label="Start"]
    exit  [shape=Msquare, label="Exit"]

    scan    [shape=parallelogram, label="Scan", script="find . -maxdepth 2 -type f | head -50"]
    analyze [shape=tab, label="Analyze", prompt="Summarize the file listing."]

    start -> scan -> analyze -> exit
}
```

## 3. Implement / Validate / Fix Loop

Use when the workflow must repair itself until tests pass.

```dot
digraph RepairLoop {
    graph [goal="Implement the change and make tests pass"]
    rankdir=LR

    start [shape=Mdiamond, label="Start"]
    exit  [shape=Msquare, label="Exit"]

    implement [label="Implement", prompt="Implement the requested change."]
    validate  [shape=parallelogram, label="Validate", script="cargo test 2>&1 || true"]
    gate      [shape=diamond, label="Passing?"]
    fixup     [label="Fixup", prompt="Read the failures and fix them.", max_visits=3]

    start -> implement -> validate -> gate
    gate -> exit  [condition="outcome=success"]
    gate -> fixup
    fixup -> validate
}
```

## 4. Plan, Approve, Implement

Use when a human should review the plan before changes land.

```dot
digraph PlanApproveImplement {
    graph [goal="Plan the work, get approval, then implement it"]
    rankdir=LR

    start [shape=Mdiamond, label="Start"]
    exit  [shape=Msquare, label="Exit"]

    plan      [shape=tab, label="Plan", prompt="Write the implementation plan to plan.md."]
    approve   [shape=hexagon, label="Approve Plan"]
    implement [label="Implement", prompt="Read plan.md and implement it."]

    start -> plan -> approve
    approve -> implement [label="[A] Approve"]
    approve -> plan      [label="[R] Revise"]
    implement -> exit
}
```

## 5. Parallel Review

Use when multiple perspectives can work independently.

```dot
digraph ParallelReview {
    graph [goal="Run a multi-perspective review"]
    rankdir=LR

    start [shape=Mdiamond, label="Start"]
    exit  [shape=Msquare, label="Exit"]

    fork         [shape=component, label="Fork", join_policy="wait_all"]
    security     [shape=tab, label="Security", prompt="Review for security issues."]
    architecture [shape=tab, label="Architecture", prompt="Review the architecture."]
    quality      [shape=tab, label="Quality", prompt="Review code quality."]
    merge        [shape=tripleoctagon, label="Merge"]
    synthesize   [shape=tab, label="Synthesize", prompt="Combine the findings."]

    start -> fork
    fork -> security
    fork -> architecture
    fork -> quality
    security -> merge
    architecture -> merge
    quality -> merge
    merge -> synthesize -> exit
}
```

## 6. Production Verification Gate

Use when the workflow must produce a must-pass final verification.

```dot
digraph ProductionGate {
    graph [goal="Implement, simplify, and verify the change"]
    rankdir=LR

    start [shape=Mdiamond, label="Start"]
    exit  [shape=Msquare, label="Exit"]

    implement [label="Implement", prompt="Implement the change."]
    simplify  [label="Simplify", prompt="Simplify the result without changing behavior."]
    verify    [shape=parallelogram, label="Verify", script="cargo clippy -- -D warnings 2>&1 && cargo test 2>&1", goal_gate=true, retry_target="fixup"]
    fixup     [label="Fixup", prompt="Fix the verification failures.", max_visits=3]

    start -> implement -> simplify -> verify
    verify -> exit  [condition="outcome=success"]
    verify -> fixup
    fixup -> verify
}
```

## Choosing Between Them

Pick the smallest pattern that answers the user's actual problem:

- one-shot thinking: linear prompt
- shell evidence plus analysis: command then analyze
- self-repairing code change: validate / fix loop
- human approval required: plan, approve, implement
- multiple independent viewpoints: parallel review
- must-pass release criteria: production verification gate

For Raspberry-supervised repos, choose the pattern only after deciding the
lane's milestone, produced artifacts, proof expectations, and observability
contract.
