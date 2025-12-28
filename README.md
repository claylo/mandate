# mandate

Mandate converts Markdown or YAML-with-Markdown into roff manpages. Think of it as a small
translator that speaks both CommonMark and `man`, with just enough schema validation to keep
things civil.

## Contents

<!-- toc -->

* [Features](#features)
* [Installation](#installation)
* [Quick start](#quick-start)
* [Usage](#usage)
* [Markdown handling](#markdown-handling)
* [CLI flags](#cli-flags)
* [Documentation](#documentation)
* [Project status](#project-status)
* [Development](#development)
* [Contributing](#contributing)
* [License](#license)

<!-- tocstop -->

## Features

- Converts Markdown or YAML-with-Markdown into roff.
- Optional schema validation for YAML manuals.
- Works with stdin for pipeline-friendly workflows.
- Tries hard to stay boring (the highest compliment for tooling).

## Installation

Prebuilt binaries:

```sh
brew install claylo/brew/mandate
```

```sh
cargo binstall mandate
```

From a local checkout:

```sh
cargo install --path .
```

Or run without installing:

```sh
cargo run -- --help
```

## Quick start

```sh
# Markdown in, roff out
mandate -i README.md -p mandate -s 1 -t "Mandate Manual"

# YAML in, roff out
mandate -i docs/mandate.yml -p mandate -s 1 -t "Mandate Manual"

# stdin if you're feeling bold (or piping)
echo '# mytool(1) -- Example tool' | mandate -i - -p mytool -s 1 -t "Mytool Manual"
```

## Usage

```text
mandate -i <input> -p <program> -s <section> -t <title> [options]
```

Notes:

- `manual.md`/`manual.markdown` → Markdown input.
- `manual.yml`/`manual.yaml` → YAML input.
- `-` reads from stdin and auto-detects format.
- `--validate` checks YAML against the embedded schema (or `--schema` override).

## Markdown handling

Mandate speaks CommonMark, but the roff renderer has opinions. Here are the ones you’ll actually trip over:

- H1 headings become the `NAME` section and are split on ` -- `, ` - `, or ` — ` into name/description (parenthesized suffixes are trimmed).
- H2 headings render as `.SH`, H3+ render as `.SS`.
- Lists with a single item ending in `:` are treated as term/definition lists; following paragraphs are indented definitions until a code block interrupts them.
- Consecutive fenced code blocks are merged into one `.nf/.fi` block to keep the roff layout tidy.
- Soft breaks become spaces; hard breaks become newlines only inside list items (outside lists they collapse to spaces too).
- Links keep their text, drop the URL. Images keep their alt text, drop the pixels.
- Block quotes are flattened (no special quoting in roff).
- Inline HTML is treated as literal text; HTML blocks become plain paragraphs.
- Tables, footnotes, strikethrough, definition lists, metadata blocks, superscripts, and subscripts are rejected with a Markdown error.
- Horizontal rules, task list markers, and math are ignored.

## CLI flags

- `-i, --input` path to `manual.yml` or `manual.md` (use `-` for stdin)
- `-p, --program` program name
- `-s, --section` man section (default: `1`)
- `-t, --title` manpage title
- `-m, --manual-section` manual section label (optional)
- `--source` source string (optional)
- `-o, --output` output file path (default: stdout)
- `--validate` validate YAML input against the built-in schema
- `--schema` path to an alternate schema to use with `--validate`

## Documentation

- Manpage source: `docs/mandate.yml`
- Decisions: `docs/decisions/`
- Changelog: `CHANGELOG.md`
- Plan/Roadmap: `PLAN.md`

## Project status

`mandate` is pre-1.0. Expect the occasional sharp edge, but nothing a comment and a sigh
can’t smooth out.

## Development

Validation:

```sh
just check
just test
just cov
```

Build the manpage used by releases:

```sh
cargo run --bin mandate -- -i docs/mandate.yml -p mandate -s 1 -t "Mandate Manual" -o target/man/mandate.1
```

## Contributing

Open a PR with a clear intent and keep changes focused. If you touch behavior, add tests in the
same change. (Future you will say thanks, and present you can pretend this was inevitable.)

## License

Licensed under either of:

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT license (`LICENSE-MIT`)

at your option.
