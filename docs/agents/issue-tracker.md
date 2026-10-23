# Issue tracker: Local Markdown

Issues and specs for this repo live as markdown files in `.scratch/`.

## Private, local only

`.scratch/` and `.out-of-scope/` are in `.gitignore`: the repo is public, so the tracker lives only in the main checkout on claude-dev. Never force-add them, and never run `git clean -x` or `-X` here: it deletes them and git keeps no copy.

Ignored files never show up as changes, so a tracker written anywhere else is thrown away without warning:

- In a git worktree, use the main checkout's folders (`$(git rev-parse --path-format=absolute --git-common-dir)/../.scratch/`), never ones inside the worktree.
- With no `.scratch/` to find (a cloud session, a fresh clone), don't create one: say so and put the ticket text in your reply, to be filed on claude-dev. Create it only once the user confirms this is claude-dev.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`
- The spec is `.scratch/<feature-slug>/spec.md`
- Implementation issues are one file per ticket at `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01`, never a single combined tickets file
- Triage state is recorded as a `Status:` line near the top of each issue file (see `triage-labels.md` for the role strings); `resolved` is its only other value
- `/triage` records an issue's category like its state, as a `Category:` line (`bug` or `enhancement`)
- Comments and conversation history append to the bottom of the file under a `## Comments` heading
- Closing an issue: set `Status:` to `resolved` once the work has landed (name the commit), or to `wontfix` with the reason under `## Comments`. The file stays.

## When a skill says "publish to the issue tracker"

Create a new file under `.scratch/<feature-slug>/` (creating the feature directory if needed; `.scratch/` itself only as "Private, local only" above allows).

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path or the issue number directly. If it isn't there, see "Private, local only" above rather than searching elsewhere.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** file per ticket.

- **Map**: `.scratch/<effort>/map.md` (the Notes / Decisions-so-far / Fog body).
- **Child ticket**: `.scratch/<effort>/issues/NN-<slug>.md`, numbered from `01`, with the question in the body. A `Type:` line records the ticket type (`research`/`prototype`/`grilling`/`task`).
- **Open / closed**: a ticket is closed once its `Status:` is `resolved` or `wontfix` (see Conventions); any other value, or no `Status:` line, is open.
- **Blocking**: a `Blocked by: NN, NN` line near the top. A ticket is unblocked when every ticket it lists is closed.
- **Frontier**: scan `.scratch/<effort>/issues/` for tickets that are open, unblocked and unclaimed, skipping `Status: needs-info` and `Status: ready-for-human` (they wait on a person); first by number wins.
- **Claim**: add a `Claimed: <date>` line and save before any work. It goes beside `Status:`, which keeps the triage role.
- **Resolve**: append the answer under an `## Answer` heading, set `Status: resolved`, then append a context pointer (gist + link) to the map's Decisions-so-far in `map.md`.
