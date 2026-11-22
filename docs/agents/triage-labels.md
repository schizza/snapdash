# Triage Labels

The skills speak in terms of five canonical triage roles.
This file maps those roles to the actual label strings used in this repo's issue tracker.

| Canonical role    | Label in our tracker | Meaning                                  |
| ----------------- | -------------------- | ---------------------------------------- |
| `needs-triage`    | `discussion`         | Maintainer needs to evaluate this issue  |
| `needs-info`      | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent` | `ready`              | Fully specified, ready for an AFK agent  |
| `ready-for-human` | `help-wanted`        | Requires human implementation            |
| `wontfix`         | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from the right-hand column.

All five labels already exist in `schizza/snapdash` - apply them with `gh issue edit <n> --add-label "..."`, never create duplicates under the canonical names.

Edit the right-hand column if the vocabulary changes.
