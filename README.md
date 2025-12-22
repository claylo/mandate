# mandate

Mandate converts Markdown/CommonMark or YAML-with-Markdown into roff manpages.

## Usage

Validation is opt-in. Mandate embeds jq's `manual_schema.yml` for YAML validation; `--schema` is only needed if you want to validate against a different schema.

Markdown input:

```sh
echo '# mytool(1) -- Example tool' | mandate -i - -p mytool -s 1 -t "Mytool Manual"
```

YAML manual input:

```sh
mandate -i manual.yml -p mytool -s 1 -t "Mytool Manual" -o mytool.1
```

## CLI flags

- `-i, --input` path to `manual.yml` or `manual.md` (use `-` for stdin)
- `-p, --program` program name
- `-s, --section` man section (default: `1`)
- `-t, --title` manpage title
- `-m, --manual-section` manual section label (optional)
- `--source` source string (optional)
- `-o, --output` output file path (default: stdout)
- `--validate` validate YAML input against the built-in schema before generating roff
- `--schema` path to an alternate schema to use with `--validate`
