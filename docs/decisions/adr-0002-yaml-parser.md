---
status: accepted
date: 2025-12-22
decision-makers: clay, codex
---

# Select yaml-rust2 for YAML parsing

## Context and Problem Statement

Mandate parses jq-style manual YAML and converts it into Markdown before
rendering roff. We need a parser that is dependable, reasonably compliant with
YAML 1.2, and stable enough to use as a general default across projects. We
also want to keep dependencies small and avoid duplicating schema validation
logic because we already validate with `jsonschema`.

## Decision Drivers

* Minimal, stable API for map/sequence access
* Low integration overhead and small dependency surface
* Solid YAML 1.2 compliance for common inputs
* Predictable maintenance posture and issue response
* Avoid writing/maintaining a custom YAML parser

## Considered Options

* rust-yaml
* yaml-rust
* yaml-rust2
* saphyr

## Decision Outcome

Chosen option: "yaml-rust2", because it provides better YAML 1.2 compliance
than `yaml-rust` while keeping the same lightweight API, and it is more mature
and dependable right now than `rust-yaml` for our inputs. It aligns with our
current conversion pipeline and keeps dependencies small.

### Consequences

* Good: Minimal API and straightforward mapping to our internal model.
* Good: No extra schema-validation features to overlap with `jsonschema`.
* Good: Improved YAML 1.2 compliance vs `yaml-rust`.
* Good: Clear upgrade path if we later switch to `saphyr` or `rust-yaml`.
* Bad: `yaml-rust2` is in basic-maintenance mode, so feature growth is limited.
* Bad: We still rely on a parser with a smaller maintainer pool than `saphyr`.

### Confirmation

* Unit tests parse jq's `manual.yml` fixture and validate schema constraints.
* YAML-to-Markdown conversion tests cover required fields and error cases.
* We do not pursue a custom YAML parser due to complexity and maintenance cost.

## Pros and Cons of the Options

### rust-yaml

* Good: Feature-rich YAML 1.2 implementation with advanced capabilities.
* Neutral: Strong focus on security and configuration, but more surface area.
* Bad: Parser correctness issues encountered on jq manual block scalars.
* Bad: PR/issue response latency made it risky as a default choice.
* Bad: Overlaps with separate schema validation using `jsonschema`.

### yaml-rust

* Good: Simple `YamlLoader` API and lightweight map/sequence access.
* Good: Minimal dependency footprint and stable, familiar API.
* Bad: Not fully compliant with YAML 1.2 test suite.

### yaml-rust2

* Good: Improved YAML 1.2 compliance over `yaml-rust`.
* Good: Drop-in replacement for `yaml-rust`.
* Bad: Maintenance posture is "basic maintenance only"; new features move to
  another crate, which suggests limited long-term evolution.

### saphyr

* Good: Fully YAML 1.2 compliant and passes the YAML test suite.
* Good: Active development, with both high-level (`saphyr`) and event-level
  (`saphyr-parser`) APIs.
* Bad: Heavier dependency footprint and additional license notices (fork
  lineage).
* Bad: API surface differs more from `yaml-rust`, so migration is less trivial.
