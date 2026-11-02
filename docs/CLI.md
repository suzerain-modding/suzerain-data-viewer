# Indexed CLI guide

## Import and storage

```powershell
.\suzerain_data_viewer.exe import 'C:/Suzerain/dumps' --database data.sqlite
.\suzerain_data_viewer.exe sources --database data.sqlite
.\suzerain_data_viewer.exe validate --database data.sqlite --check-files
```

`import` accepts one or more files or directories. Directories contribute their
immediate `.json` files, sorted deterministically; subdirectories and HTML are not
read. Every input must be a supported dump. Keep unrelated JSON outside the input
directory, or pass individual filenames.

This version reads the **legacy object-keyed format**, with roots
`<conversations Sordland>`, `<conversations Rizia>`, `<entity data>` and
`<actor names>`. It does not implement the future Dumper v2 export format.
UTF-8 BOMs are accepted. Inputs are read, never edited.

The default index path is `suzerain-data.sqlite` in the current directory. Its parent
directory must exist. SQLite and FTS5 are bundled into the executable. The index
stores decoded original node/entity records, fields, scripts, edges, entity names,
game-flow locations and full-text search data. Allow substantially more disk space
than the source JSON size. A whole source JSON file is parsed in memory during
import, one file at a time; query commands read the index rather than reparsing dumps.
Conversation records contain their header; their child nodes are stored separately.
The index schema is internal and can change; use CLI results as the supported interface.

Index version 2 keeps dialogue fields in each original node record rather than
duplicating them in a separate table. The former `totals.fields` statistic is removed.
Version 1 indexes must be reimported into a new database path; there is no migration
or dependency on the removed field table. Raw field values and query views are preserved.

Imports append captures. A source with the same canonical path and SHA-256 is
skipped. A changed file at the same path becomes a separate source; older indexed
captures remain queryable. Source IDs from `sources` disambiguate these captures.
Files with identical content at different paths remain separate sources.

`import --replace` replaces all indexed sources, within the same transaction. It
accepts only a Viewer index with the supported index version, or a new/empty SQLite
file. Arbitrary databases, input dumps and incompatible index versions are refused.
For an incompatible index version, import into a new database path.
Any parse, identity, schema, indexing or import-summary size error rolls back the
entire import, including a requested replacement. Source changes detected during
import also cause failure. Import from completed dumps rather than files still being written.

## Queries

All query commands accept `--database`, `--source ID`, `--pack Sordland|Rizia`,
`--format json|text|markdown`, `--output FILE` and `--max-bytes N`.
`StoryPack_Main` and `StoryPack_Rizia` are also accepted pack names.
Entity pack filtering uses exported bundle membership, which can include both packs.
The legacy names-only actor export has no pack membership.

| Command | Purpose |
| --- | --- |
| `sources` | Source paths, hashes, root types, sizes, import times and counts |
| `validate [--check-files]` | Structural/import warnings, duplicate Articy IDs, unresolved links and optional source hash verification |
| `search [TEXT] [--kind all\|conversations\|nodes\|entities]` | Literal full-text phrase in title/text/scripts, plus title/name substring; omit TEXT to list records |
| `node --conversation-id C --entry-id E` | Compact dialogue node with text, menu text, actor IDs, exact Lua, sequences, links and entity context |
| `node --articy-id ID [--conversation-id C]` | Exact Articy ID lookup, case insensitive |
| `neighbors <node selector> [--depth 1..5] [--direction incoming\|outgoing\|both]` | Bounded graph traversal, shortest hop distances and raw outgoing links |
| `references TEXT [--language lua\|articy\|sequence\|variable]` | Case-sensitive literal matches in indexed script fields |
| `entity NAME [--kind KIND]` | Exact entity name lookup, original properties, scripts, schedule and conversation matches |

`node` and `entity` also accept `--raw` to return the original decoded record plus
provenance. Missing and null fields, empty strings, duplicate field titles and
decoded script whitespace are preserved in raw records. JSON formatting and escape
spelling are not byte-for-byte copies of the input file. Compact views expose
duplicate field values as ambiguous rather than choosing one. Duplicate `Articy Id`
fields disable exact Articy lookup for that node; use its numeric identity and raw view.

`entity --kind` uses these case-sensitive names: `BillData`, `ConversationData`,
`DecisionData`, `ConditionalInstructionData`, `GameFlowData`, `NewsData`, `ActorName`.
Game-flow record names are their exported `StoryPack` values (for example,
`StoryPack_Main`). Search first when an exact name is unknown.

Node IDs are scoped to their source and conversation. Articy IDs and entity names
can also repeat. Exact lookup refuses ambiguity; add `--source`, `--conversation-id`
or `--kind` as appropriate. Graph traversal never crosses source exports, even when
another capture contains an apparently matching target. Outgoing links retain raw
IDs, priority and connector flags, including targets absent from the export.
Traversal is bounded by depth and unique node/depth pairs, then paginated; a highly
connected graph may still take longer than a simple lookup.

References report textual matches, **not** inferred execution or read/write roles.
Lua conditions/user scripts, articy conditions/instructions, and sequences retain
their language labels. There is no script execution or translation.

## Provenance and relationships

Each record carries its source ID, canonical file path, SHA-256, root type and JSON
Pointer. Source metadata separates Viewer import time from unknown capture time.
Legacy exports do not contain game/schema versions or reliable capture metadata;
those values are explicitly `null`. A directory of dumps is not treated as a
simultaneous snapshot, and importing together does not create that guarantee.

ConversationData joins dialogue conversations by exact exported title. The results
include both sources' provenance and identify the relationship as a title match
across independent captures. Fragment-to-game-flow joins use names within the same
entity export. `turn_list_index` and `step_list_index` are raw list indices, **not**
displayed game turn/step numbers. An unmatched join means absent export coverage,
not proof that an object is unused or that a game bug exists.

Actor names are stored as `ActorName` records, with an `export_ordinal` and
`actor_id: null`. The names-only export does not map ordinals to runtime actor IDs.
Dialogue nodes retain their raw actor/conversant IDs and leave `actor_name: null`.

`source.html` points to the conventional sibling directory named after the input
file stem. Conversation links include `#DialogueEntry-ID`; entity links currently
target the category page. `html_exists` checks the file, not the fragment or capture
match. HTML generated elsewhere is not automatically discovered; source pointers
and raw records work without HTML.

## Output and paging

Query stdout contains only the requested response. Progress and errors go to stderr.
JSON is the default. Text and Markdown formats wrap the same structured JSON with
a heading (and a Markdown code fence), preserving the same information.
`--output FILE` creates a **new** UTF-8 file; existing files are refused, with no
overwrite option. It cannot name the database. Parent directories must exist.

`search`, `sources`, `validate`, `neighbors` and `references` accept `--limit 1..200`
(default 20) and `--cursor`. A response's `page.next_cursor` continues that exact
query. `page.truncated` means more results exist; a null cursor means the page is
complete. Ordering is deterministic within an unchanged index. Cursors are bound
to the query, source/pack filters and index revision; another query or a successful
import that changes the index invalidates them. Read commands use a SQLite read
transaction to keep each response consistent.

```powershell
.\suzerain_data_viewer.exe references GovernmentBudget --pack Sordland --limit 20
# Copy the returned page.next_cursor into the same query:
.\suzerain_data_viewer.exe references GovernmentBudget --pack Sordland --limit 20 --cursor 'CURSOR'
.\suzerain_data_viewer.exe node --pack Sordland --conversation-id 1 --entry-id 105 --raw --output node105.json
```

`--max-bytes` caps the complete encoded response, default 262144 bytes, range
1024..16777216. Paged queries remove whole tail records until the response fits and
adjust the cursor so no results are skipped. A single oversized record or fixed
details block causes an explicit error; increase the limit or choose another view.
Raw records and script values are never silently cut to fit. Import summaries are
not paginated; exceeding this cap rolls back the import.

Search/neighbor/reference summaries have 400-character text excerpts, labelled
`excerpt_truncated`. Reference matches have 800-character excerpts and up to 20
matching script locations per record. Node links are capped at 200; entity/dialogue
matches, entity script locations and schedule locations are capped at 20 each.
Every nested cap has an explicit `truncated` flag. Use exact/raw lookup or source
JSON Pointers for complete records when nested results are truncated.

Validation counts all warnings in the selected sources even when the warning list
is paginated; `warning_counts` groups them by code. Unresolved links appear first.
Reused Articy IDs indicate lookup ambiguity, not necessarily invalid game data.
Pack selection chooses sources containing that pack, so validation
of a shared entity export includes its other-pack records too. Overall totals and
foreign-key integrity describe the entire index. `--check-files` verifies all
selected source hashes, up to 200 sources; narrow with `--source` for larger indexes.
An unresolved link is a warning about export coverage. Warnings do not fail the
command; foreign-key or requested file-integrity failures do.

Exit status is 0 for success, 1 for operational/validation failure, 2 for invalid
CLI arguments. Validation integrity failures still emit their structured response.

## HTML conversion

```powershell
.\suzerain_data_viewer.exe html --input conversations.json --output html
.\suzerain_data_viewer.exe html --input conversations.json --output html --overwrite
```

No arguments retains the current-directory JSON picker and `out/` default. The
explicit `html` command defaults to `out/` too and never prompts. Conversation and
entity exports render through the existing templates; actor-name exports are
available through the index instead. HTML progress is human-readable.

An existing output requires `--overwrite` and a valid `.sdv-output.json` ownership
marker from this Viewer version. Legacy unmarked output is refused. Pages first
render into a fresh temporary sibling directory. Replacement copies generated
files, removes obsolete generated files from the previous marker and preserves
unrelated files. New pages conflicting with unrelated files are refused. Output
symlinks and non-file destinations are refused. Do not edit the ownership marker.
After conflict checks, the marker atomically claims the old and new generated paths
before any pages are copied. A failed copy or obsolete-page deletion leaves that
ownership list intact, so the same command can be retried after fixing the I/O error.
Successful regeneration narrows the marker to the final generated file list.
Rendering errors leave previous output intact; an I/O failure during replacement
can leave partially updated pages, so rerun the command after resolving the failure.
Malformed items now fail conversion with context and a nonzero exit status rather
than silently producing a partial site, including nested entity collections and
invalid string-list elements. Nullable dialogue field values render as `(null)`,
distinct from `(empty)` strings. Source dumps are never modified.
