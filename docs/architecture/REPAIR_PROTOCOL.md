# Repair protocol

A repair contract is both human-readable and machine-readable.

Required fields:

- issue ID;
- problem;
- why it matters;
- evidence anchors;
- required fix;
- behavior to preserve;
- acceptance criteria;
- suggested re-checks;
- forbidden shortcuts where useful.

Example:

```yaml
problem: "The app reports that an email was sent, but the real send path only logs to the terminal."
why_it_matters: "Users are told an action succeeded when no email leaves the app."
required_fix:
  - "Call the configured email provider on the real send path."
  - "Handle provider rejection without exposing secrets."
preserve:
  - "Current successful UI flow."
acceptance:
  - "Provider is called on successful send."
  - "Provider failure is handled and tested."
```

After repair, SURE selects affected + regression checks. Agent self-report does not close the finding.
