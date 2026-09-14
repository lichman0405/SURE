---
name: sure-check
description: Use the local SURE checker to verify an AI-built project, preserve unknowns, and apply/re-check SURE repair contracts.
---

# SURE project checking

When asked to verify a project:

1. Confirm the `sure` CLI is available; if not, do not pretend it ran.
2. Run the appropriate `sure check` command for the project.
3. Preserve SURE statuses exactly: pass/fail/warning/skipped/error/unknown and confirmed/contradicted/cannot-confirm.
4. Explain important findings in plain language.
5. If the user chooses a repair, use SURE's repair contract as the acceptance contract.
6. After making the repair, invoke SURE re-check. Your own completion message is not evidence that the finding is resolved.
