Your task is to verify that the develop-web-game skill is properly set up.

1. Call the `use_skill` tool with skill_name "develop-web-game" to load the skill instructions.
2. Read the skill instructions and identify the asset file paths referenced in them.
3. Verify the following files exist and are readable:
   - `skills/develop-web-game/scripts/web_game_playwright_client.js`
   - `skills/develop-web-game/references/action_payloads.json`
   - `skills/develop-web-game/SKILL.md`
4. Read the contents of `action_payloads.json` and confirm it contains valid JSON with a "steps" array.
5. Read the first 10 lines of `web_game_playwright_client.js` and confirm it imports playwright.
6. Verify that `npx playwright --version` works (playwright should be available after the install step).
7. Write a summary report to `output/skill-verification.md` with the results.

Report success or failure for each check.
