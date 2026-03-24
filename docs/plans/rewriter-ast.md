# AST Round-Trip Spec Rewriter with Trivia Preservation

## Overview

The spec rewriter uses an AST-based approach with full trivia (comment)
preservation. The lexer captures comments in a side channel, a `TriviaMap`
associates comments with AST nodes by proximity, and the rewriter operates on
typed AST nodes — replacing `PlaceDecl` spans with formatter-generated text
that includes re-attached trivia. The parser's existing `Spanned<T>` byte
offsets provide precise replacement targeting.

Approach: **full trivia-preserving AST round-trip**.


## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|----------------|
| Side-channel comment capture (not inline tokens) | Emitting comment tokens inline would require parser changes to skip them everywhere → every `expect(Token::...)` call becomes fallible → massive refactor → side-channel avoids parser changes entirely while capturing all comment data |
| TriviaMap as separate structure (not fields on AST nodes) | Adding trivia fields to every AST node type requires modifying ~20 struct definitions → TriviaMap keyed by Span allows trivia attachment without any AST type changes → existing AST round-trips unchanged → trivia is opt-in for consumers |
| Leading/trailing trivia association by byte proximity | Comments before a node are "leading" trivia for that node; inline comments after a node's closing token are "trailing" → this matches developer intuition → rustfmt and prettier use the same heuristic → unambiguous for 95%+ of cases |
| Orphan trivia preserved via span gaps | Comments between replaced spans and the next AST node must not be lost → the rewriter copies source text verbatim for all byte ranges not covered by a replacement → orphan comments fall in these gap ranges → preserved automatically |
| Formatter generates replacement text (not string concatenation) | String concatenation for `place` block generation is error-prone (indentation, separators, quoting) → the existing `fmt_place_decl()` already handles all formatting correctly → reuse it with trivia injection → single source of truth for spec syntax |
| Rewriter stays in altium-cli (not autopcb-spec) | The rewriter depends on `PlacementResult` from `autopcb-placement` → `autopcb-spec` has no dependency on `autopcb-placement` → adding one creates a cycle → `altium-cli` sits at the top of the dependency graph and can import both |
| Replace entire PlaceDecl span (not property-level surgery) | Property-level replacement (only replacing `autoplace: true` line) requires tracking individual property spans within the Object body → fragile if properties span multiple lines or use spread syntax → replacing the entire PlaceDecl span is simpler and the formatter regenerates clean output |
| Multi-designator expansion at AST level | The old rewriter did multi-designator expansion via text manipulation → AST approach: detect `place C1, C2 { autoplace: true }` as a PlaceDecl with multiple designators → generate N separate PlaceDecl AST nodes → format each → emit as replacement text |
| `lex()` returns comments alongside tokens (not Lexer struct threading) | The lexer is a standalone `fn lex(input) -> Vec<Token>` function, not a stateful struct threaded through the parser → modifying `lex()` to return `(Vec<Token>, Vec<CommentToken>)` is a one-line signature change → `parse_with_trivia()` calls `lex()`, takes the comments, passes tokens to the parser → zero parser API changes needed |
| TriviaMap enumerates only PlaceDecl spans (not full AST) | The rewriter only replaces PlaceDecl nodes → enumerating all AST nodes adds complexity without benefit → comments outside PlaceDecl spans fall in untouched gap ranges and are preserved verbatim → TriviaMap only needs to associate comments with PlaceDecl spans for trivia re-attachment |
| TriviaMap uses two BTreeMaps: leading keyed by node start, trailing keyed by node end | Leading trivia (comments before a node) is looked up by node start byte → BTreeMap range query finds preceding comments → trailing trivia (same-line comment after node end) is looked up by node end byte → separate key space avoids collision between leading and trailing |
| Unsolved multi-designator blocks get `// autoplace: unsolved` annotation | Preserving original body verbatim AND adding an annotation are contradictory → chose annotation because it's more informative → the body content (`autoplace: true` plus other properties) is preserved, but a trailing comment is appended to signal the component was not solved → user can re-run autoplace to solve remaining components |
| Indentation detected by scanning backward from span start to newline | For each PlaceDecl being replaced, count the space/tab characters between the most recent newline and `span.start` → this gives the column offset of the `place` keyword → pass as base indent to the formatter → tabs count as 1 character (matching formatter convention) |
| Unit tests over proptest for rewriter | Spec rewriting has complex input/output relationships where property-based testing would require generating valid spec files AND valid PlacementResults simultaneously → the invariant "every comment byte preserved" is better tested via specific fixture inputs with known comment positions → proptest would need a custom spec+result generator that doesn't yet exist |
| Extract formatter's trivia types to shared trivia.rs (not duplicate) | The formatter already has `ItemTrivia`, `TriviaLine`, and `scan_trivia_lines()` for comment handling → duplicating this in a new `TriviaMap` creates two parallel trivia representations in the same crate → extract formatter's private trivia types to `trivia.rs` as `pub(crate)`, import them back into `formatter.rs` → single source of truth for trivia representation → `TriviaMap` wraps the shared types with span-indexed lookup |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| Span-indexed surgical replacement without trivia (Approach A) | User explicitly chose full trivia preservation → comments inside `place {}` bodies would be lost without trivia capture |
| Hybrid span + inline comment scan (Approach C) | Heuristic comment detection is unreliable for block comments and edge cases → full trivia capture is more robust and not much more code |
| Moving rewriter to autopcb-spec | Creates dependency cycle with autopcb-placement → would require extracting PlacementResult into a shared types crate → over-engineering for one consumer |
| CST (Concrete Syntax Tree) approach | Full CST requires every token (including whitespace) to be represented in the tree → massive parser rewrite → side-channel trivia is 90% of the benefit with 10% of the cost |
| Modifying AST node structs to carry trivia fields | Would require changing ~20 struct definitions and all their constructors → TriviaMap achieves the same result without touching AST types |

### Constraints & Assumptions

- The lexer already tracks byte positions for all tokens via `Span { start: u32, end: u32 }`
- `Spanned<T>` wraps every AST node with byte offsets into source
- The formatter's `fmt_place_decl()` can generate canonical `place` block text
- Comments use `//` (line) and `/* */` (block, with nesting) syntax — nested block comments already supported by lexer (depth counter at lexer.rs:164, test at lexer.rs:952)
- The rewriter has exactly one call site: `autoplace_spec()` in `main.rs`
- Parser tests must continue to pass unchanged (lexer change is additive)
- `Coord` and `CoordPoint` types have `.to_mms()` for coordinate conversion

### Known Risks

| Risk | Mitigation | Anchor |
|------|-----------|--------|
| Trivia association ambiguity between two adjacent nodes | Use "attach to following node" rule (leading trivia) with fallback to "trailing on previous node" for same-line comments → matches rustfmt convention | N/A (new code) |
| Formatter output doesn't match original indentation level | Detect indentation of the span being replaced (first non-whitespace column) and pass as base indent to formatter → formatter already accepts indent level | crates/autopcb-spec/src/formatter.rs:171-209 (Printer struct with indent tracking) |
| Comment inside a `place` property value (rare but possible) | Property values are single-line expressions in the spec grammar → comments can only appear between properties, not inside them → not a realistic concern | crates/autopcb-spec/src/lexer.rs:155-188 (comment skipping in lexer) |
| Tab-indented files produce space-normalized output | The scan-backward character count treats tabs as 1 character; the formatter generates spaces at `indent_level * config.indent` width → a 2-tab-indented block produces `indent_level=2` → 8 spaces at default `indent=4` → cosmetic mismatch, recoverable by running the formatter on the full file | N/A (formatter design choice) |


## Invisible Knowledge

### Architecture

```
Source Text (.pcb)
         │
    ┌────┴────┐
    │  Lexer  │──→ Token Stream (unchanged)
    │         │──→ Vec<CommentToken>  (NEW: side channel)
    └────┬────┘
         │
    ┌────┴─────┐
    │  Parser  │──→ AST with Spanned<T> byte offsets (unchanged)
    └────┬─────┘
         │
    ┌────┴──────────┐
    │  TriviaMap    │  NEW: associates comments with AST nodes
    │  Builder      │  by scanning Vec<CommentToken> + AST spans
    └────┬──────────┘
         │
    ┌────┴──────────┐
    │  AST Rewriter │  NEW: walks AST, finds autoplace PlaceDecls
    │               │  builds replacement text via formatter + trivia
    └────┬──────────┘
         │
    ┌────┴──────────┐
    │  Span Replacer│  replaces byte ranges in source text
    │               │  reverse-order to preserve offsets
    └────┬──────────┘
         │
         ▼
Updated Source Text
```

### Data Flow

```
parse_with_trivia(source)
  → (SpecFile, TriviaMap)
       │           │
       ▼           ▼
  find PlaceDecls  lookup trivia for each PlaceDecl span
       │           │
       ▼           ▼
  for each autoplace PlaceDecl:
    1. Build new PlaceDecl AST node with solved at:/rotation:
    2. Format via fmt_place_decl() → replacement text
    3. Prepend leading trivia, append trailing trivia
    4. Record (span.start, span.end, replacement_text)
       │
       ▼
  apply replacements in reverse byte order
       │
       ▼
  output text (non-replaced ranges preserved verbatim)
```

### Why This Structure

The trivia system is decoupled from the AST and parser:
- **TriviaMap is external** to AST nodes. The parser requires no new fields or
  constructor arguments; all 262 parser/compiler tests run unchanged.
- **The lexer stores comments in a side channel, not as tokens.** The token
  stream the parser consumes is the same token stream it has always consumed.
  Only the side channel carries comment data.
- **The rewriter consumes both AST and TriviaMap** but neither the parser nor
  the compiler needs to know about trivia. This is a clean separation: trivia
  is a concern of source-level tooling (rewriter, formatter), not semantic
  processing (compiler, executor).

### Invariants

- Every byte of the original source text appears in exactly one of: (a) an
  untouched gap between replacements, or (b) a replacement range. No bytes
  are silently dropped.
- Comments that fall within a replaced PlaceDecl span are re-attached to the
  replacement text via TriviaMap. Comments outside all replaced spans are
  preserved verbatim in the gap ranges.
- After rewriting, the output can be re-parsed without errors. This is
  enforced by a roundtrip test.
- Multi-designator `place C1, C2 { autoplace: true }` blocks are expanded
  at the AST level before formatting, ensuring each component gets its own
  block with correct trivia attachment.
- The annotation strings `// autoplace: solved` and `// autoplace: unsolved`
  are stable user-facing markers. Downstream tooling may parse these strings
  to identify placement status from spec files. These strings must not be
  renamed without coordinating with consumers.

### Tradeoffs

- **TriviaMap vs AST-embedded trivia**: TriviaMap requires a separate lookup
  per node (O(log N) with sorted comment list) but avoids modifying any AST
  types. AST-embedded trivia would be O(1) lookup but requires changing every
  AST struct. At spec file scale (typically <200 comments), the performance
  difference is negligible.
- **Full PlaceDecl replacement vs property-level surgery**: Replacing the
  entire PlaceDecl means the formatter decides formatting, not the original
  source. Cost: original author's formatting preferences within `place {}`
  blocks are normalized to canonical style. Benefit: dramatically simpler
  implementation (no per-property span tracking needed).
- **Comments between properties lose their exact association**: A comment
  like `// this resistor must be near U1` between `region: center` and
  `autoplace: true` will be re-emitted as leading trivia for the first
  property in the regenerated block. The comment is preserved but may
  shift position relative to surrounding properties.


## Milestones

### Milestone 1: Lexer Comment Capture + TriviaMap

**Files**:
- `crates/autopcb-spec/src/lexer.rs`
- `crates/autopcb-spec/src/trivia.rs` (new)
- `crates/autopcb-spec/src/lib.rs`

**Flags**: `conformance`

**Requirements**:
- Lexer captures all comments (line and block) in a `Vec<CommentToken>` side channel during tokenization
- `CommentToken` struct: `span: Span`, `text: String`, `is_block: bool`
- The token stream consumed by the parser is unchanged (comments still skipped for parsing)
- New `parse_with_trivia(source: &str) -> Result<(SpecFile, TriviaMap), SpecError>` function
- `TriviaMap` struct built from `Vec<CommentToken>` + AST node spans
- `TriviaMap::leading(span: Span) -> &[CommentToken]` — comments immediately before a node
- `TriviaMap::trailing(span: Span) -> Option<&CommentToken>` — inline comment after a node on same line
- `TriviaMap::in_range(start: u32, end: u32) -> &[CommentToken]` — all comments within a byte range

**Acceptance Criteria**:
- `// line comment` captured with correct span and text
- `/* block comment */` captured with correct span, text, and `is_block: true`
- Nested `/* outer /* inner */ */` captured as single block comment
- All 262 existing parser tests pass unchanged
- `TriviaMap::leading()` returns the comment before a `place` block
- `TriviaMap::trailing()` returns inline comment on same line as closing `}`

**Tests**:
- **Test files**: `crates/autopcb-spec/src/trivia.rs` (inline `#[cfg(test)]`)
- **Test type**: unit
- **Backing**: default-derived
- **Scenarios**:
  - Normal: spec with line comments → all captured with correct spans
  - Normal: spec with block comments → captured with is_block=true
  - Edge: empty spec → empty TriviaMap
  - Edge: comment at EOF with no following node → orphan trivia
  - Normal: comment immediately before `place` block → leading trivia
  - Normal: `} // trailing` → trailing trivia for that node
  - Normal: comment between two `place` blocks → leading trivia for second block

**Code Intent**:
- `lexer.rs`: `pub fn lex(input: &str)` returns `(Vec<Token>, Vec<CommentToken>)`. The lexer is a standalone function (not a struct) — a local `comments: Vec<CommentToken>` variable accumulates comment tokens during tokenization. In the line-comment branch (line 155), captures `CommentToken { span, text, is_block: false }` before advancing past the comment. In the block-comment branch (line 161), captures `CommentToken { span, text, is_block: true }` after the depth-tracking loop completes. Nested block comments are handled by the depth counter at line 164. All callers of `lex()` destructure the tuple — callers that only need tokens use `let (tokens, _comments) = lex(input)?`.
- `trivia.rs` (new): Extract formatter's existing `ItemTrivia`, `TriviaLine`, and `scan_trivia_lines()` from `formatter.rs` into this module as `pub(crate)` types. `formatter.rs` imports them back. Add `CommentToken { span: Span, text: String, is_block: bool }`. `TriviaMap` struct with two internal maps:
  - `leading: BTreeMap<u32, Vec<CommentToken>>` keyed by the start byte of the AST node each comment precedes. Two maps (separate key spaces) avoid collision between leading and trailing lookups.
  - `trailing: BTreeMap<u32, CommentToken>` keyed by the end byte of the AST node the comment follows on the same line.
  - `TriviaMap::build(comments: Vec<CommentToken>, ast: &SpecFile) -> TriviaMap` — enumerates only PlaceDecl spans (not the full AST tree: the rewriter only replaces PlaceDecl nodes, so full enumeration adds complexity without benefit). For each comment, binary-searches the sorted PlaceDecl span starts to find the nearest following node; same-line comments after a node's end byte go into trailing. Comments not near any PlaceDecl are not attached — they fall in untouched gap ranges and are preserved verbatim.
  - `parse_with_trivia(source: &str) -> Result<(SpecFile, TriviaMap), SpecError>` — calls `lex()` to get `(tokens, comments)`, passes tokens to the existing parser, builds TriviaMap from comments + AST.
- `lib.rs`: Add `pub mod trivia;` and re-export `CommentToken`, `TriviaMap`, `parse_with_trivia`.

---

### Milestone 2: AST-Based Spec Rewriter

**Files**:
- `crates/altium-cli/src/spec_rewriter.rs` (rewrite)
- `crates/altium-cli/src/main.rs` (update call site)

**Flags**: `needs-rationale`, `conformance`

**Requirements**:
- `rewrite_spec_with_placement()` implemented using AST + TriviaMap
- Parses source via `parse_with_trivia()` to get AST with spans + TriviaMap
- Walks AST to find `PlacementDecl`, iterates its `PlacementItem::Place` children
- For each `PlaceDecl` with autoplace designators:
  - Builds a new `PlaceDecl` AST node with `at:` and `rotation:` properties from solver result
  - Formats using `fmt_place_decl()` from the formatter
  - Collects leading/trailing trivia from TriviaMap for this PlaceDecl's span
  - Records `(span.start, span.end, replacement_text_with_trivia)` as a replacement
- Multi-designator `place C1, C2 { autoplace: true }` expanded to individual PlaceDecl nodes at AST level
- Unsolved designators in multi-designator blocks: regenerate with original body properties (including `autoplace: true`) plus a `// autoplace: unsolved` trailing comment to signal the component was not solved
- Appends new `place` blocks for autoplace components not in the original source
- Replacements applied in reverse byte order to preserve offsets
- All source text outside replacement spans preserved verbatim (including comments, whitespace)
- Detects indentation level of each replaced span and passes to formatter

**Acceptance Criteria**:
- `place U1 { autoplace: true }` → `place U1 { at: (x, y), rotation: N } // autoplace: solved`
- Comments before a `place` block preserved in output
- Comments inside a `place` block body preserved in output (re-attached as leading trivia)
- Trailing `// comment` on closing `}` preserved
- Comments between non-autoplace blocks preserved exactly (byte-for-byte)
- Output re-parses without errors (roundtrip test)
- Locked components (no autoplace) appear unchanged in output
- Multi-designator expansion produces individual blocks with correct trivia
- Unsolved designators keep `autoplace: true` with `// autoplace: unsolved` annotation

**Tests**:
- **Test files**: `crates/altium-cli/src/spec_rewriter.rs` (inline `#[cfg(test)]`)
- **Test type**: unit
- **Backing**: doc-derived
- **Scenarios**:
  - Normal: autoplace replaced, locked preserved, comments preserved
  - Normal: comment before place block → preserved in output
  - Normal: comment inside place body → preserved in output
  - Normal: trailing comment on `}` → preserved
  - Normal: multi-designator expanded with trivia
  - Edge: no placement block in spec → output identical to input
  - Edge: all components locked → output identical to input
  - Roundtrip: rewrite → re-parse → verify no parse errors
  - Edge: comment between two place blocks → attached to correct block

**Code Intent**:
- `spec_rewriter.rs`: Implementation based on AST + TriviaMap. Top-level entry:
  1. `rewrite_spec_with_placement(source, result, autoplace_designators) -> RewriteResult` — main entry point
  2. Parse source with `parse_with_trivia()` → `(ast, trivia_map)`
  3. `find_placement_decl(&ast) -> Option<&Spanned<PlacementDecl>>` — locate placement block
  4. `collect_replacements(placement, &trivia_map, result, autoplace_designators) -> Vec<Replacement>` — iterate PlaceDecl items, build replacement entries
  5. `build_replacement_text(place_decl, solver_state, &trivia_map, indent) -> String` — format a single PlaceDecl with trivia. Indentation: scan backward from `span.start` to the most recent newline, count leading space/tab characters (tabs count as 1, matching formatter convention) to determine the column offset, pass as `indent_level` to the formatter. This preserves the original nesting depth even when `place` blocks appear inside outer structures.
  6. `apply_replacements(source, replacements) -> String` — apply in reverse byte order
  7. `build_append_text(unmatched_designators, result) -> String` — generate blocks for components not in source
  8. Multi-designator handling: if PlaceDecl has >1 designator and any is autoplace, expand to individual PlaceDecl AST nodes, format each separately
- `main.rs`: `autoplace_spec()` calls `rewrite_spec_with_placement()` at the autoplace call site

---

### Milestone 3: Documentation

**Delegated to**: @agent-technical-writer (mode: post-implementation)

**Source**: `## Invisible Knowledge` section of this plan

**Files**:
- `crates/autopcb-spec/CLAUDE.md` (update: add trivia.rs entry)
- `crates/autopcb-spec/README.md` (update: document trivia system)
- `crates/altium-cli/CLAUDE.md` (update: spec_rewriter.rs description)

**Requirements**:
- Update CLAUDE.md indexes to reflect new trivia.rs module
- Document the trivia architecture in README.md (from Invisible Knowledge)
- Update spec_rewriter.rs description in altium-cli CLAUDE.md

**Acceptance Criteria**:
- CLAUDE.md is tabular index only (no prose sections)
- README.md documents trivia system architecture and invariants
- README.md is self-contained


## Milestone Dependencies

```
M1 (Lexer + TriviaMap) ──→ M2 (AST Rewriter) ──→ M3 (Docs)
```

Sequential chain — M2 depends on M1's `parse_with_trivia()` and `TriviaMap`.
M3 is documentation-only, runs after implementation.
