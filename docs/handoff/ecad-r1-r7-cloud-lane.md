# Cloud lane brief: altium-format R1–R7 for ecadabra (batch 1, lane C)

Written 2026-10-23 by the batch coordinator (the cloud session the user named coordinator for
ecadabra and cadatomic batch 1). This brief and `docs/handoff/ecad-r1-r7.md` (the previous
handoff, same branch) are your instructions.

## Where you work

- **Your branch** is your session's `outcome_branch`, `claude/ecad-b1-altium-r1-r7`. Push only
  there. Open no pull requests. Never push to `master` or `ecad-r1-r7`.
- **Base:** `ecad-r1-r7` `d7fef24` (R2 work in progress) plus this brief, on branch
  `claude/ecadabra-cloud-parallel-tjzgmf`. That line forks from altium-cli master `18062d6`, the rev
  ecadabra's importer pins. Don't rebase onto a newer master.
- **The user approved fixing R1–R7** ("ECAD lane fixes them", 2026-10-22) on a branch for their
  review. The result is reviewed by the user before it merges anywhere.
- **`CLAUDE.md` binds in full**: fail fast, no opaque or raw retention, no skipped data, domain
  types from `altium-format-types` (constants checked against the decompiled sources where they are
  available; in the cloud they are not, so say which constants you could not check), errors with
  context, privacy of altium-format internals.
- **Completeness review:** `CLAUDE.md` asks for an independent agent's completeness review after
  every step. After each R-item, spawn one review subagent on that item's diff, fix what it finds,
  and record its verdict in the report.

## Setup

1. `hostname`, `nproc`, `free -g`, `df -h .`: record them. Stop if `/mnt/c` exists.
2. Toolchain: rustc as installed (record it).
3. Corpus: clone `https://github.com/akiselev/altium-cli-test-pcbdoc` shallow to
   `/tmp/corpus/pcbdoc`, check out `6bc5aa7` if you can (record HEAD otherwise), and link
   `data/pcbdoc` to it (`data/` is ignored; never commit corpus files). PcbLib likewise if an item
   needs it (`c5dfdc7`).
4. The sandbox may reject compound shell commands, python heredocs and paths built from variables
   inside the repo. Use one plain command per call, or the Write tool. The previous lane's survey
   scripts (python with olefile) were outside the repository and are not available; write your own
   under `/tmp`, never in the repo.
5. Build with `CARGO_TARGET_DIR=/tmp/target-altium` and `CARGO_PROFILE_DEV_DEBUG=0`.

## Tests

`CLAUDE.md` says fixture and proptest tests run only when explicitly asked. **The coordinator asks
for exactly these:** the corpus test of "Next steps" item 2 in `ecad-r1-r7.md`, and one filtered
fixture test per R-item that checks it on the corpus, each run filtered to its own name
(`cargo test -p altium-format --features test-fixtures <name>`). No full fixture suite, no
proptests. Unit tests (no feature) as usual.

## Work, in order

`ecad-r1-r7.md` "Next steps" 1–5: test and finish R2, then R4, R3, R1, R6, R5, R7, one commit
each, pushed as you go. The request texts are in that file's predecessor,
ecadabra `docs/spikes/e2-altium-import.md` (quoted in `ecad-r1-r7.md` where it matters; ecadabra
is private, so you may not be able to read it, and you don't need to). Where a request asks for
something `CLAUDE.md` forbids (R2's `remaining_params`), type it instead and say so.

API breaks in altium-format-spec and altium-cli struct literals are approved (fill defaults);
list them as an API break in the report.

## Report

`docs/handoff/ecad-r1-r7-cloud-report.md`, committed and pushed as the last action, also
replacing the state table in `ecad-r1-r7.md`:
- branch and final sha;
- per request: state, public API added (as code), evidence, corpus counts (files that read
  before and after; no board that read before may fail now);
- tests run, as counts with the exact commands;
- review-agent verdicts per item;
- API breaks;
- questions for the coordinator or the user, each with a recommendation;
- what ecadabra's importer must change to use each field (one line each), for the ecadabra lane
  that moves the pin;
- the setup as it actually ran.

Near 400k tokens of context, stop, commit, and write the handoff into the report.
