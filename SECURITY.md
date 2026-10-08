# Security policy

## Reporting a vulnerability

Please **don't** open a public issue for a security problem. Report it privately instead:

- on GitHub: **Security → Report a vulnerability** in this repository (private vulnerability reporting), or
- by email to the maintainer, Jonas Hollund Tjernsmo (see the GitHub profile [@jhtjernsmo](https://github.com/jhtjernsmo)).

Include what you found, how to reproduce it, and which version and platform you used. You'll get a
reply within 7 days. Once a fix is released, the report is credited in the release notes unless
you'd rather stay anonymous.

## Supported versions

Fixes go into the latest release. Fjord updates itself, so please update before reporting.

## How Fjord handles security

- Data stays on your machine in one SQLite database; there are no accounts and no telemetry.
- GitHub and Azure DevOps tokens are kept in the operating system's credential store, never in
  Fjord's database or files.
- Updates are signed, and verified before they're installed. The macOS app is signed with an Apple
  Developer ID and notarized.
- The MCP server has no tools that hard-delete data, and every change records who made it.
- External commands (git, the editor) run without a shell, so file paths can't inject commands.
