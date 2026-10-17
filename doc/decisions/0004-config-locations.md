# 0004 Configuration locations

## Context

Version 1 read a single `./config.toml` from the working directory and wrote
saved presets to the same place. That works when running from the project root
but gives no stable place for personal presets, and it silently ignored a typo
in an explicit `--config` path.

## Decision

Layer configuration with `directories::ProjectDirs`, from lowest to highest
precedence:

1. Built-in presets, compiled in.
2. The user config, `ProjectDirs::from("", "", "halftone").config_dir()/config.toml`
   (`~/.config/halftone/config.toml` on Linux).
3. The project config, `./config.toml`, when present.

Later layers override earlier ones, including `default_preset` when the later
file sets it. Saving a preset targets the user config by default, creating the
directory as needed. An explicit `--config PATH` replaces layers 2 and 3 and a
missing explicit file is an error.

## Alternatives considered

- **Single file only.** Simple, but no stable per-user location and no way to
  share project presets without collision. Rejected.
- **User config overriding project config.** Personal settings would silently
  defeat a project's chosen defaults, which is surprising when working inside a
  repository. Rejected.
- **Merge preset patches field by field across layers.** More flexible but makes
  it hard to reason about the effective value. Chose whole-patch replacement by
  preset name instead.
- **Write the user config from the platform config directory as TOML with the
  built-ins materialized.** Keeps one self-documenting file per layer. Accepted;
  the file is larger but fully editable.

## Consequences

- `~/.config/halftone/config.toml` is the natural home for personal presets and
  survives moving between projects.
- The repository's `config.toml` acts as the project layer: running from the
  repo root lets it override personal defaults, which is intended for a sample
  project but worth remembering.
- `preset save` no longer writes into the current directory by default. This is
  a behavior change; `--config PATH` restores explicit control.
- A typo in `--config` now fails fast instead of silently using built-ins.
- Platform paths come from `directories` (Linux `$XDG_CONFIG_HOME`, macOS
  `Application Support`, Windows `%APPDATA%`). `preset path` reports them, so
  the behavior is discoverable.
