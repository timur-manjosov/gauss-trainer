# Changelog

All notable changes to this project are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Release workflow that builds binaries for Linux (x86_64, aarch64), macOS
  (Apple Silicon, Intel) and Windows (x86_64) and attaches them with a
  `SHA256SUMS` file to the GitHub release of every `v*` tag.

## [0.1.0] - 2026-10-18

First release.

### Added

- Exact rational arithmetic (`BigRational`) for every computation; no
  floating point anywhere in the crate.
- The three elementary row operations — multiply a row, add a multiple of a
  row to another, swap two rows — with undo, redo, reset to A₀ and new
  matrices. Factors may be integers, fractions (`-3/4`) or decimals
  (`0.25`, `0,25`), converted exactly.
- Rechenprotokoll in the wording of the FernUniversität Maple trainer with
  the A₀ → A₁ → A₂ … numbering.
- Command bar with history, short and long command forms (also German
  aliases), Ctrl shortcuts and a `?` help overlay.
- Learning features: live analysis (pivots, pivot columns, entries that
  still have to become 0, zero rows, rank, status), task presets, hints in
  three levels on request, a confirmation-gated reference solution, check
  mode for hand-computed matrices and an exam mode with timer.
- Typeset matrices with stacked fractions, tall brackets and true minus
  signs; dark and light theme; truecolor with 256-colour fallback; adaptive
  layout with scrolling; clear message below 80×24.
- Reproducible exercises through `--seed` and the `new seed=K` command;
  start matrices via CLI argument or in the UI.
- Test suite with unit, property (proptest), snapshot (insta) and
  end-to-end tests; GitHub Actions workflow (fmt, clippy, tests on Linux,
  macOS and Windows).

[0.1.0]: https://github.com/timur-manjosov/gauss-trainer/releases/tag/v0.1.0
