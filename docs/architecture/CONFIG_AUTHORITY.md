# Configuration authority

The checked project may be controlled by the same AI whose work SURE is evaluating. Therefore project-controlled configuration is not trusted to grant SURE additional authority.

## Authority order

Highest authority:
1. explicit interactive/user approval for the current action;
2. user-level SURE configuration stored outside the project;
3. organization policy (future team edition, where applicable);
4. project `sure.yaml` suggestions;
5. inferred defaults.

A lower-authority source cannot weaken a higher-authority safety/privacy restriction.

## Project config may

- select/disable noncritical checks;
- describe project components;
- provide explicit project goal/spec paths;
- suggest start/test commands;
- configure report preferences.

## Project config may not silently

- enable host execution if the user did not allow it;
- enable dependency installation/network access;
- disable user protection rules;
- enable full transcript recording;
- provide/override secret credentials as an authority bypass;
- delete authoritative history.

If project config requests a privileged behavior, SURE treats it as a request requiring higher-authority approval.
