# Branching

Everything reaches `main` through `dev`.
`main` is the release branch and only ever moves when a release is cut.

## The rule

- Branch from `dev`, never from `main`.
- Every PR targets `dev`, whatever it carries: feature, fix, refactor, chore, docs, dependency bump.
- `main` receives exactly one kind of PR, the release PR that promotes `dev` once the work accumulated there has been tested and is stable.

So a change lands twice: once into `dev` when it is written, and again into `main` when it ships.
That second hop is what a version number means in this repo.

## Consequences worth knowing

`main` lags `dev`, sometimes by several features.
Reading `main` to find out what the code does now is wrong; read `dev`.

A branch cut from `main` will be missing whatever is sitting in `dev`, and rebasing it onto `dev` afterwards is a merge conflict you did not need to have.
Check `git log --oneline main..dev` before branching if you are unsure how far apart they are.

Local `origin/*` refs go stale if `git fetch` is failing.
`git fetch` runs over SSH and will refuse when the key is locked, while the `gh` CLI keeps working over its token, so the tracker can look current while the branches do not.
`git ls-remote https://github.com/schizza/snapdash.git refs/heads/dev refs/heads/main` answers the question without needing the key.

## Where the base branch is set

`gh pr create --base dev`.
GitHub's repository default branch is `main`, so the base is not inferred correctly and has to be passed.
