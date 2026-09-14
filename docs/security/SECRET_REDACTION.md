# Secret redaction

Redaction must occur before user-visible reports and preferably before durable recording/model context.

Sources:
- explicitly configured secret names/values;
- environment-variable names likely to contain secrets;
- common token/key formats;
- provider credentials.

Requirements:
- avoid logging raw environment values;
- redact stdout/stderr before persistent full-recording write where feasible;
- test false positives as well as known secret patterns;
- never claim perfect secret detection.
