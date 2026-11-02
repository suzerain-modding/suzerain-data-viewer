# P1 manual validation

Validated on Windows x64 with the pinned Rust 1.95.0 toolchain on 2026-11-02.
These are recorded observations against the available legacy dumps, not expected
counts for other game versions. No automated test project or framework was added.
Linux builds remain covered by the existing release build workflow and were not
run locally for this change.

## Build checks

- `scripts/preflight.ps1` passed, including MSVC and Windows SDK discovery.
- `cargo fmt --check` passed.
- `cargo clippy --locked -- -D warnings` passed with incremental caching disabled
  in the restricted workspace (`CARGO_INCREMENTAL=0`).
- Locked debug and release builds passed. SQLite/FTS5 compiled from bundled source;
  no game, interop assemblies or .NET SDK were used.

## Real dump import and queries

The four legacy exports imported into a disposable index with these totals:

| Item | Count |
| --- | ---: |
| Sources | 4 |
| Conversations | 512 |
| Dialogue nodes | 200,663 |
| Outgoing links | 253,215 |
| Fields retained in original node JSON | 2,060,888 |
| Script locations | 87,236 |
| Game-flow fragment locations | 522 |
| Actor-name records | 103 |
| Bills / conversations / decisions | 26 / 264 / 310 |
| Conditional instructions / game flows / news | 368 / 2 / 1,476 |

The version 2 index occupied approximately 624 MiB, down from approximately 740 MiB
before removing the unused field table and index (116 MiB saved on these dumps).
`validate --check-files` confirmed all
four original hashes and zero foreign-key failures. It reported 24,432 groups of
reused Articy IDs within conversations and one unresolved link to conversation
391, entry 1 in the Rizia export. These are coverage/lookup diagnostics, not proof
of invalid game data. The warning list is paginated, with totals by code.

Sordland conversation 1, entry 105 returned its exact decoded Lua script, including
the trailing space. Numeric and unambiguous Articy lookup returned the same record.
Its immediate neighbors were entries 342, 343 and 344. The associated
`Turn01_EnT_EconomicOverview` entity joined to `StoryPack_Main`, raw turn index 0,
step index 1. Entry 3 exposed English dialogue and sequence fields while retaining
actor ID 10 and an unknown actor name.

Paginating all Sordland `GovernmentBudget` references produced 322 distinct records:
279 nodes, 15 bills, 2 conditional instructions and 26 decisions. Exact lookup of a
reused Articy ID failed with an ambiguity message. Focused node/entity/neighbor
lookups took roughly 20-40 ms; reference pages took roughly 0.4 seconds on this
machine with warm filesystem caches. These timings are observations, not guarantees.

## CLI and overwrite checks

Disposable input files and indexes confirmed help for every subcommand, clean JSON
stdout, text/Markdown output, source/pack selection, language filters, literal FTS
input quoting, bounded pages, cursor continuation without skipped/duplicate rows,
cross-query and stale cursor rejection, byte-limit errors and invalid argument exits.

Import checks covered unchanged-source skipping, UTF-8 BOMs, successful replacement,
duplicate numeric identities, duplicate field titles, malformed inputs and oversized
import summaries. Failed replacements/imports retained the previous index revision.
Source hash mismatches produced structured validation failure and a nonzero exit.
Original dump paths, unrelated databases, existing response files and a response
path equal to the new database path were refused as overwrite targets.

Both full conversation dumps and the full entity export rendered through the HTML
command. The Sordland and Rizia runs generated all 286 and 226 conversation pages
plus their indexes/assets. Sample node anchors were checked in generated HTML.
Marked regeneration preserved unrelated notes, removed obsolete generated pages
when switching export types, and refused unmarked directories or collisions with
unrelated pages. Invalid JSON and a malformed dialogue child failed before replacing
the existing site. The interactive picker was preserved in code; terminal interaction
with that picker was not separately exercised.

## Follow-up review fixes

After the review fixes, a fresh version 2 import retained the same node, edge,
script and game-flow counts, and the full-corpus queries above passed again.
The removed field table/index were absent from SQLite's schema; the original raw
entry 105 matched the earlier import exactly. Version 1 indexes were refused with
guidance to import into a new database path. Import and response output checks
continued to protect input files, existing response files and a new response path
equal to the database path after consolidating destination resolution.

Twenty-three focused manual CLI checks covered nullable fields, invalid field
types, overwrite recovery, nested rendering failures and destination protection.
Setting the old index page read-only reproduced a failure after a new conversation
page was copied. Clearing that attribute and retrying succeeded with the new
ownership marker. A read-only obsolete page also produced a deletion failure that
could be retried successfully. A read-only marker failed before publishing pages;
unrelated files and conflicting page names remained protected.

Null field values rendered as `(null)`, empty strings as `(empty)`, and nonempty
strings remained HTML-escaped. Numeric field values were rejected. Invalid nested
Turns, Steps, Fragments, StoryPacks, Options and ConditionalInstructions collections,
plus a non-string fragment item, all returned nonzero status with context. Hashes
of every previously generated entity file stayed unchanged after those failures.
The full entity export and both conversation exports rendered successfully after
propagating these errors. No test framework or test project was introduced.

Input JSON, generated HTML, indexes and probe outputs are local artifacts and are
not included in the repository change. See [CLI.md](CLI.md) for usage and limits.
