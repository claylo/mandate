---
status: accepted
date: 2025-12-22
decision-makers: clay, codex
---

# Select yaml-rust for YAML parsing

## Context and Problem Statement

Mandate parses jq-style manual YAML and converts it into Markdown before
rendering roff. We initially considered `rust-yaml` for its YAML 1.2 feature
set, but we need a stable, minimal API that matches the jq manual schema and
keeps dependencies small. We also want to avoid duplicating schema validation
logic because we already validate with `jsonschema`.

## Decision Drivers

* Minimal, stable API for map/sequence access
* Low integration overhead and small dependency surface
* Sufficient YAML 1.2 support for jq's manual schema
* Clear maintenance posture and predictable upgrades

## Considered Options

* rust-yaml
* yaml-rust
* yaml-rust2

## Decision Outcome

Chosen option: "yaml-rust", because it provides a simple, well-known API that
fits the jq manual schema with minimal integration effort, without pulling in
extra features we do not need. It aligns with our current conversion pipeline
and keeps dependencies small.

### Consequences

* Good: Minimal API and straightforward mapping to our internal model.
* Good: No extra schema-validation features to overlap with `jsonschema`.
* Good: Easy to swap to `yaml-rust2` later if compliance issues surface.
* Bad: Less YAML 1.2 compliance than `yaml-rust2` or the feature-rich `rust-yaml`.

### Confirmation

* Unit tests parse jq's `manual.yml` fixture and validate schema constraints.
* YAML-to-Markdown conversion tests cover required fields and error cases.

## Pros and Cons of the Options

### rust-yaml

* Good: Feature-rich YAML 1.2 implementation with advanced capabilities.
* Neutral: Strong focus on security and configuration, but more surface area.
* Bad: Newer API and additional features not needed for jq manuals.
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
