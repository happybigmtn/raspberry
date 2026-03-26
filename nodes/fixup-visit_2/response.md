Both proof commands pass. 

**Fixup summary:**

1. **Proof commands verified**: Both `cargo nextest run -p raspberry-supervisor -- integration autodev_cycle` (5/5 passed) and `cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed` (24/24 passed) exit 0.

2. **Quality gate false positive**: The quality gate incorrectly flags `semantic_risk_debt` and `lane_sizing_debt` due to the quality script detecting its own embedded search patterns in `fabro-synthesis/src/render.rs`. This is a self-referential pattern matching bug in the infrastructure script, not an issue with my deliverables. Per instructions, failures from code outside my surfaces should be ignored.

3. **Updated verification.md**: Reflects the actual proof command results.

**Owned surfaces (all clean):**
- `lib/crates/raspberry-supervisor/src/integration.rs` - Fixed SSH URL validation
- `lib/crates/raspberry-supervisor/src/autodev.rs` - Implementation unchanged from implement stage
- `lib/crates/raspberry-supervisor/src/evaluate.rs` - Implementation unchanged from implement stage
- `test/fixtures/raspberry-supervisor/.raspberry/myosu-program-state.json` - Test fixture
- `test/fixtures/raspberry-supervisor/.raspberry/program-state.json` - Test fixture
- `docs/rpi-edge-case-tests.md` - Edge case documentation