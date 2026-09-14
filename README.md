# SURE

**Software Understanding & Reality Evaluation**

> **AI says it's done. Be SURE.**

SURE is a local-first project checking layer for software built with AI coding tools.

It helps an individual builder or a small team answer:

1. **Can this project actually work?**
2. **Is anything clearly broken, unsafe, fake or incomplete?**
3. **Can the AI's claims about what it completed be confirmed?**

When SURE finds a real problem, it explains the consequence in plain language, creates a bounded repair contract, hands that contract back to the user's existing coding agent, and checks again after the repair.

## Product rule

> **Never replace the harness. Sit beside it.**

Claude Code users keep using Claude Code. Cursor users keep using Cursor. Codex users keep using Codex.

## Two modes

### Check this project

Works even if SURE did not observe the development session.

It can inspect the current project, run approved checks, identify obvious incomplete/fake implementation and produce a plain-language report.

It **cannot** claim that the project matches the user's original request unless that request/goal was supplied or observed.

### Full-session checking

With a supported harness plugin/hook, SURE also records selected development facts and can compare the AI's completion claims with what actually happened.

## Current repository status

This repository contains an autonomous-development bootstrap, not the completed SURE product.

Start with `START_HERE.md`.

## Development bootstrap

Primary v0.1 development host: **Windows 11 x64 / native MSVC**. See `START_HERE.md`.
