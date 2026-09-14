# UX and language

## User mental model

Not:

> run software provenance verification

But:

> **Check what the AI actually built.**

## Primary actions

- Check this project
- Turn on full-session checking
- Let AI fix
- Check again
- Show technical details

## Severity

- **Must fix** — do not recommend publishing/hand-off.
- **Should fix first** — material reliability/quality risk.
- **Can fix later** — non-blocking improvement.
- **Note** — informational.

## Evidence wording

Use:

- Confirmed
- Contradicted
- Cannot confirm
- Not checked
- Needs a real external service to verify

Do not turn uncertainty into a confidence percentage and call it truth.

## Requirements wording

If SURE did not observe or receive the user's original requirement, say so:

> "I can check whether the current project runs and whether anything obviously looks incomplete. I cannot confirm that it matches your original request because that request was not provided to SURE."

## Jargon rule

Do not lead with:

- SHA mismatch
- provenance
- gate
- attestation
- policy violation
- schema drift

Translate consequence first.

Bad:

> Schema drift detected.

Good:

> The database structure changed, but there is no matching database update file. A new installation may fail or use the wrong structure.
