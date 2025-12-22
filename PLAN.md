# Mandate Plan

## Goals
- Single crate with `src/lib.rs` + `src/main.rs`.
- Convert Markdown/CommonMark or YAML-with-Markdown into roff manpage output.
- CLI flags: `-i/--input`, `-p/--program`, `-s/--section`, `-t/--title`, `-m/--manual-section`, `--source`, `-o/--output`.
- Port jq’s `build_manpage.py` algorithm + YAML processing (from jq dev manual) where applicable.
- Track jq documentation licensing in `COPYING.md`.

## Reference
- ../reference/jq/docs/build_manpage.py
- ../reference/jq/docs/content/manual/dev/manual.yml
- https://man7.org/linux/man-pages/man7/man-pages.7.html

## Non-goals (initial)
- Multiple crates/workspace split.
- Full jq docs feature parity beyond the ported algorithm.
- Advanced formatting beyond what jq’s builder emits.

## Phase 1 — Repo scaffolding
- Create `Cargo.toml` for a single crate (lib + bin) with Rust 2024 edition and `#![forbid(unsafe_code)]`.
- Add dependencies: `pulldown-cmark` (per `docs/decisions/adr-0001-markdown-parser.md`), `yaml-rust`, and a CLI parser (use `clap` with derive).
- Add `src/lib.rs` with public API for conversion and shared types.
- Add `src/main.rs` for CLI entrypoint and IO plumbing.

## Phase 2 — Input model + parsing
- Define a minimal internal model for manpage content mirroring jq’s builder.
- Implement Markdown parsing using `pulldown-cmark`, mapping tokens to the internal model.
- Implement YAML parsing using `yaml-rust`, matching jq’s dev manual schema; extract Markdown sections and metadata.
- Validate YAML manuals against the embedded schema in `data/manual_schema.yml` with `jsonschema` when `--validate` is set (allow override via `--schema`).
- Support stdin for `--input` when `-i` is `-` or absent (decide default and document).

## Phase 3 — Manpage rendering
- Port jq’s manpage rendering rules from `build_manpage.py`.
- Implement roff output generation with deterministic whitespace and escaping rules.
- Ensure `--program`, `--section`, `--title`, `--manual-section`, and `--source` correctly map to header fields.

## Phase 4 — Fixtures + tests
- Add `COPYING.md` with jq MIT + CC-BY-3.0 documentation note and links.
- Vendor jq’s `manual.yml` into `tests/fixtures/`.
- Unit tests for Markdown conversion edge cases (headings, lists, code blocks, emphasis).
- Unit tests for YAML-to-Markdown mapping using the fixture.
- CLI integration test that exercises file + stdin input and output file writing.
- Integration test that verifies the full end-to-end conversion from YAML + Markdown to roff manpage output.
- Integration test that verifies correct handling of CLI flags and header field mapping.
- Integration test that verifies correct handling of stdin input when `-i` is `-` or absent.
- Integration test that verifies correct handling of output file writing when `-o` is specified.
- Integration test that verifies correct handling of both stdin input and output file writing simultaneously.
- Integration test that verifies correct handling of error cases (invalid input, missing required flags, etc.).
- Integration test that verifies correct handling of edge cases in Markdown and YAML input.
- Integration test that verifies correct handling of large input files and performance characteristics.
- Achieve 100% test coverage.

## Phase 5 — Docs + polish
- Add README usage example and CLI help snippet.
- Add thorough crate-level rustdoc with examples.
- Verify clippy/format and ensure tests are deterministic.

## Validation
- `just fmt`
- `just clippy`
- `just test`
- `just doc-test`
- `cargo check --all-targets --all-features`
