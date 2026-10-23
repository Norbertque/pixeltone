# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

This repo's tracker is local markdown (`issue-tracker.md`): the label string goes in the `Status:` line of each issue file.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

`Status:` takes one value besides these: `resolved`, which closes an issue once its work has landed (`issue-tracker.md`). Treat it as closed, like `wontfix`; it is not a role to triage. Wayfinder claims go on a separate `Claimed:` line, never in `Status:`.

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.
