Done. Created `.fabro-work/contract.md` defining DONE for this lane.

**Summary:**

- **5 deliverables:** `implementation.md`, `verification.md`, `quality.md`, `promotion.md`, `integration.md` — all in `.fabro-work/`
- **5 acceptance criteria** (all verifiable by command or file inspection):
  1. `fabro synth create` exits 0 on tonofcrap
  2. Scaffold stage is a dependency of every non-scaffold stage in the generated graph
  3. `raspberry autodev --max-cycles 30` completes without infrastructure-caused failures
  4. No silent `any[]`-type-gate false-pass after scaffold completes
  5. Bootstrap verification gate is present in the rendered workflow graph
- **Out of scope:** All implementation work (scaffold ordering, bootstrap gate, asset resolution, type-aware quality) is owned by sibling lanes; tonofcrap is a read-only validation target; no source changes from this lane