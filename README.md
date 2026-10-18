# gauss-trainer

**A terminal trainer for the Gaussian algorithm.** You choose every elementary row operation, the computer does the arithmetic — exactly, in fractions — and shows what is still missing on the way to the *Treppennormalform*.

It reproduces the "MapleApp" row-operation trainer of the course *Mathematische Grundlagen* at the FernUniversität in Hagen as a fast, offline terminal application. The interface is in German and uses the terminology of the course; code, documentation and commit messages are in English.

[![CI](https://github.com/timur-manjosov/gauss-trainer/actions/workflows/ci.yml/badge.svg)](https://github.com/timur-manjosov/gauss-trainer/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Demo

This is the real output of the program (rendered with ratatui's `TestBackend` at 100×28; a test keeps this block in sync with the code). Mid-solve of the course example with a level-2 hint requested. The pivot `1` and the entry `7` that still has to become `0` are coloured green and red in the terminal, the changed row is tinted:

```text
 ╭ Matrix · A₄ ──────────────────────────────────── 3×4 ╮╭ Rechenprotokoll ─────────── 4 Schritte ╮
 │                                                      ││   A₀       Startmatrix                 │
 │                                                      ││   A₀ → A₁  Das (-2)-fache von Zeile    │
 │                   1   2       3     4                ││            1 wird zu Zeile 2           │
 │              ▎  ⎡            1     5  ⎤              ││            addiert.                    │
 │              ▎1 ⎢ 1   0     ───   ─── ⎥              ││   A₁ → A₂  Das (-3)-fache von Zeile    │
 │              ▎  ⎢            3     3  ⎥              ││            1 wird zu Zeile 3           │
 │                 ⎢                     ⎥              ││            addiert.                    │
 │                 ⎢            4     4  ⎥              ││   A₂ → A₃  Zeile 2 wird mit dem        │
 │               2 ⎢ 0   1   − ───   ─── ⎥              ││            Faktor 1/3 multipliziert.   │
 │                 ⎢            3     3  ⎥              ││ ▸ A₃ → A₄  Das 2-fache von Zeile 2     │
 │                 ⎢                     ⎥              ││            wird zu Zeile 1 addiert.    │
 │                 ⎢                     ⎥              ││                                        │
 │               3 ⎢ 0   7      −7     6 ⎥              ││                                        │
 │                 ⎣                     ⎦              ││                                        │
 │                                                      ││                                        │
 │                                                      ││                                        │
 │                                                      ││                                        │
 │                                                      ││                                        │
 │ ■ Pivot = 1  ■ Pivot ≠ 1  ■ muss 0 werden            ││                                        │
 ╰──────────────────────────────────────────────────────╯╰────────────────────────────────────────╯
  Rang r = 3  ·  noch offen: 1 Eintrag ≠ 0 in Pivotspalte                      3×4 · eigene Matrix
  ◆ Tipp 2/3: Spalte 2: Über oder unter dem Pivot in Zeile 2 steht noch ein Eintrag ≠ 0. Hier hilft
    das Addieren eines Vielfachen einer Zeile zu einer anderen.
 ╭ Befehl ────────────────────────────────────────────────────────────────────────────────────────╮
 │ ❯ m 1 -1/2  ·  a 8 1 2  ·  s 1 2  ·  ? für die Hilfe                                           │
 ╰────────────────────────────────────────────────────────────────────────────────────────────────╯
  Enter ausführen  ↑↓ Verlauf  Ctrl+Z Zurück  Ctrl+Y Vor  Ctrl+R Reset       ? Hilfe  Ctrl+Q Ende
```

> No animated GIF is included: `vhs` and `asciinema` were not installed on the machine this was built on, and the build deliberately did not install system software.

## Features

**Practising**

- The three elementary row operations — multiply a row by a nonzero factor, add the *c*-fold of a row to another, swap two rows — with **exact rational arithmetic** (`BigRational`). There is no floating point anywhere in the crate; entries are always shown as fractions.
- Factors may be integers, fractions such as `-3/4`, or decimals such as `0.25` / `0,25`, converted exactly (`0.1 + 0.2` really is `0.3` here).
- **Undo, redo, reset** to A₀ and new matrices. Reset is "undo everything", so it can itself be redone.
- The **Rechenprotokoll** uses the wording of the Maple tool ("Zeile 1 wird mit dem Faktor -1/10 multipliziert.", "Das 8-fache von Zeile 1 wird zu Zeile 2 addiert.", "Zeile 1 wird mit Zeile 2 vertauscht.") and numbers the steps as on paper, A₀ → A₁ → A₂ …; the current index is shown in the matrix title. Undone steps stay visible, dimmed.
- Random m×n matrices (1…6 each, default 3×4) with small integer entries, reproducible through `--seed`; or your own matrix via CLI argument or in the UI.

**Learning**

- **Live analysis** (toggle): pivot positions (green if already 1, amber if not yet 1), pivot columns, entries in pivot columns that still have to become 0 (red), zero rows, the rank *r*, and a status line — "Treppennormalform erreicht ✓" or what is still missing.
- **Task presets**: generic, `a₁₁ = 0` (a swap is needed), a linearly dependent row (a zero row appears), more columns than rows, more rows than columns.
- **Hints on demand only**, in three escalating levels: 1 a guiding question, 2 the kind of operation, 3 the exact operation and the command that enters it. They are computed from your *current* matrix, however you got there.
- **Reference solution**, shown only after an explicit confirmation.
- **Check mode**: type in your hand-computed matrix; the trainer tells you whether it equals the current matrix or the unique Treppennormalform and lists the differing entries.
- **Exam mode**: no highlights, no hints, no live analysis, no check, no reference solution — and a visible timer.

**Interface**

- Matrices are typeset: stacked fractions (numerator, bar, denominator), right-aligned columns, tall Unicode brackets, a true minus sign (−).
- Dark and light theme, truecolor with an automatic 256-colour fallback, rounded borders.
- Adapts to the terminal: stacked fractions when there is room, inline `a/b` otherwise, scrolling when even that does not fit, and a clear message below 80×24.

## Installation

You need a Rust toolchain (1.88 or newer to build the program; the test suite's dev-dependencies need 1.89).

```sh
# straight from GitHub
cargo install --git https://github.com/timur-manjosov/gauss-trainer

# or from a checkout
git clone https://github.com/timur-manjosov/gauss-trainer
cd gauss-trainer
cargo install --path .
# ...or just run it in place
cargo run --release
```

Terminal requirements: a Unicode-capable terminal. Truecolor is detected through `COLORTERM` (override with `--colors`). The tall brackets `⎡ ⎢ ⎣` live in the "Miscellaneous Technical" Unicode block, which nearly all monospaced fonts cover; if yours does not, use `--simple-brackets`.

## Usage

```sh
gauss-trainer                          # random 3×4 exercise
gauss-trainer --seed 42                # reproducible exercise
gauss-trainer --size 4x5 --preset abhängig
gauss-trainer "[[1,-2,3,-1],[2,-1,2,2],[3,1,2,3]]"   # your own matrix
gauss-trainer "1 -2 3 -1; 2 -1 2 2; 3 1 2 3"         # the same, flat notation
gauss-trainer --matrix "-1 2; 3 4"     # a matrix starting with a minus sign
gauss-trainer --exam --theme hell
```

### Command-line options

| Option | Meaning |
|--------|---------|
| `[MATRIX]` / `--matrix <MATRIX>` | start matrix; nested `[[1,2],[3,4]]` or flat `1 2; 3 4` |
| `-s`, `--seed <N>` | seed for reproducible exercises |
| `--size <MxN>` | size of the generated matrix, 1…6 per dimension (default `3x4`) |
| `-p`, `--preset <ART>` | `allgemein` · `a11` · `abhängig` · `breit` · `hoch` |
| `--theme <NAME>` | `dunkel` (default) · `hell` |
| `--exam` | start directly in exam mode |
| `--no-live` | start with the live analysis switched off |
| `--colors <MODE>` | `auto` (default) · `truecolor` · `256` |
| `--simple-brackets` | use `┌ │ └` instead of `⎡ ⎢ ⎣` |
| `-h`, `--help` / `-V`, `--version` | help / version |

### Commands

Type them in the command bar at the bottom. Rows count from 1; commands are case-insensitive.

| Command | Aliases | Meaning |
|---------|---------|---------|
| `m <i> <c>` | `mul`, `mal` | multiply row *i* by the nonzero factor *c* |
| `a <c> <i> <j>` | `add` | add the *c*-fold of row *i* to row *j* (*i* ≠ *j*) |
| `s <i> <j>` | `swap`, `tausch` | swap rows *i* and *j* |
| `u` | `undo`, `z` | undo the last step |
| `r` | `redo`, `y` | redo the last undone step |
| `reset` | | back to A₀ (redo brings the steps back) |
| `n [MxN] [Art] [seed=K]` | `new`, `neu` | new exercise; `Art` is one of the presets above |
| `set [Matrix]` | `e`, `eingabe` | use your own matrix; without argument a prompt opens |
| `check [Matrix]` | `c`, `prüfen` | compare a hand-computed matrix; without argument a prompt opens |
| `t` | `tipp`, `hint` | next hint level for the current matrix |
| `lösung` | `loesung`, `solution` | reference solution (asks first) |
| `analyse` | `live` | toggle the live analysis |
| `exam` | `prüfung` | toggle exam mode |
| `theme` | `thema` | toggle dark/light |
| `v` | `ansicht`, `view` | cycle the layout: automatic · stacked · inline |
| `?` | `h`, `help`, `hilfe` | help overlay |
| `q` | `quit`, `exit`, `ende` | quit |

**Numbers:** `3`, `-3/4`, `0.25`, `0,25`, `.5`, and the true minus `−`. Invalid input produces a German message, never a panic. **Matrices:** `[[1,-2],[3,4]]` or `1 -2; 3 4` (rows separated by `;`, `|` or a newline). In the flat notation a comma is a *decimal* comma, so separate entries with spaces there.

### Keys

| Key | Action |
|-----|--------|
| `Enter` | run the command |
| `Esc` | clear the line / cancel a prompt / close an overlay |
| `↑` `↓` | recall earlier commands (scroll inside overlays) |
| `←` `→` `Home` `End` `Ctrl+A` `Ctrl+E` | move the cursor |
| `Ctrl+U` `Ctrl+W` | clear the line / delete the word before the cursor |
| `Ctrl+Z` `Ctrl+Y` | undo / redo |
| `Ctrl+R` | reset to A₀ |
| `Ctrl+N` | new exercise (same size and preset) |
| `F2` or `Ctrl+G` | next hint level |
| `F3` | check a hand-computed matrix |
| `F4` | exam mode on/off |
| `F5` | cycle the matrix layout |
| `Ctrl+L` | live analysis on/off |
| `Ctrl+T` | dark/light theme |
| `PgUp` `PgDn`, `Ctrl+←↑↓→` | scroll the matrix if it does not fit |
| `?` or `F1` | help overlay (`?` only on an empty line) |
| `Ctrl+Q` or `Ctrl+C` | quit |

## Mathematical background

### Elementary row operations

Three operations change a matrix without changing what it "says":

| Operation | Effect | Inverse |
|-----------|--------|---------|
| multiply row *i* by *c* ≠ 0 | *zᵢ ← c·zᵢ* | multiply by 1/*c* |
| add the *c*-fold of row *i* to row *j* ≠ *i* | *zⱼ ← zⱼ + c·zᵢ* | add the (−*c*)-fold |
| swap rows *i* and *j* | *zᵢ ↔ zⱼ* | itself |

Each is left-multiplication by an invertible matrix, so every operation can be undone by another one. Consequently they preserve the row space, the **rank**, and — if you read the last column as the right-hand side of a linear system — the solution set.

### Treppennormalform (reduced row echelon form)

A matrix is in *Treppennormalform* iff

1. all zero rows are at the bottom,
2. the first nonzero entry of every nonzero row — its **pivot** — is 1,
3. the pivots of consecutive nonzero rows move strictly to the right, and
4. each pivot column contains zeros except for its pivot.

Every matrix can be brought into this form by elementary row operations (the Gauß–Jordan algorithm below), and the result is **unique**. Sketch of why: row operations do not change the row space *U*. Let the pivot columns of a Treppennormalform *R* of *A* be *j₁ < … < jᵣ*. A nonzero vector of *U* is a combination of the rows of *R*, and its first nonzero entry sits in the pivot column of the first row used — so the "leading columns" occurring in *U* are exactly *j₁ … jᵣ*. The *k*-th row of *R* is a vector of *U* with leading column *jₖ* and zeros in all other pivot columns. If *v* and *w* were two such vectors, *v − w* would lie in *U* and have zeros in all pivot columns; being nonzero would force a leading column outside *{j₁ … jᵣ}* — contradiction. Hence *v = w*, so *R* depends on *U* alone, i.e. on *A* alone. The **rank** *r* is the number of nonzero rows of *R*.

### The reference algorithm and its tie-breaking

The built-in solver (used for hints, the reference solution and the exam verdict) is a deterministic Gauß–Jordan elimination. With `r` pivots found so far, for each column `c` from left to right:

1. Among the rows `r…m` with a nonzero entry in column `c`, choose the **topmost row whose entry is exactly 1** (no scaling and no fractions needed); if there is none, the topmost nonzero entry. Columns without a candidate are skipped.
2. Swap the chosen row into position `r` if necessary.
3. Multiply row `r` by the reciprocal of the pivot unless it is already 1.
4. Add the `(−a_ic)`-fold of row `r` to every other row `i` — above *and* below — with `a_ic ≠ 0`.

Steps that would change nothing are never emitted, which makes the solver continue gracefully from any partially reduced matrix. Cost: at most `min(m,n)` pivots with `O(m)` steps of `O(n)` rational operations each. Exact arithmetic means no rounding errors; coefficient growth is harmless at these sizes.

### Example

The course example, read as the linear system `x₁ − 2x₂ + 3x₃ = −1`, `2x₁ − x₂ + 2x₃ = 2`, `3x₁ + x₂ + 2x₃ = 3`:

```
⎡ 1  −2  3  −1 ⎤        ⎡ 1  0  0   15/7 ⎤
⎢ 2  −1  2   2 ⎥   →    ⎢ 0  1  0   −4/7 ⎥      x = (15/7, −4/7, −10/7)
⎣ 3   1  2   3 ⎦        ⎣ 0  0  1  −10/7 ⎦
```

## Terminology

| Deutsch (UI) | English |
|--------------|---------|
| Zeile / Spalte | row / column |
| Zeilenoperation | elementary row operation |
| vertauschen · multiplizieren · addieren | swap · multiply (scale) · add |
| Faktor, das *c*-fache | factor, the *c*-fold |
| Pivotstelle (Pivot) | pivot position (pivot) |
| Pivotspalte | pivot column |
| Nullzeile | zero row |
| Rang *r* | rank |
| Treppennormalform | reduced row echelon form (RREF) |
| Rechenprotokoll | calculation log |
| Startmatrix A₀ | initial matrix |
| Musterlösung | reference solution |
| Tipp | hint |
| Prüfungsmodus | exam mode |
| linear abhängig | linearly dependent |
| Lösungsmenge | solution set |

## Architecture

A library plus a thin binary (`src/main.rs`). The core modules perform no I/O and know nothing about the user interface; `ui` only renders state.

```text
main.rs ─► cli, tui ─► app ─► parse ─► ops ─► matrix ─► rational
                        │  └► generator, hint, check ─► echelon ┘
                        └──────── ui (reads App, never mutates it)
```

| Module | Responsibility |
|--------|----------------|
| `rational`, `matrix` | `BigRational` scalar, bounded dense matrix (1…6 × 1…6), invariants documented |
| `ops` | `RowOperation` enum: validated `apply`, `inverse`, German `describe`, `to_command` |
| `echelon` | analysis (pivots, pivot columns, entries to clear, zero rows, issues), rank, `is_rref`, the reference solver |
| `generator` | SplitMix64 PRNG, exercise presets, reproducible by seed |
| `hint`, `check` | three-level hints from the solver; comparison with the current matrix and the Treppennormalform |
| `parse` | numbers, matrices, commands — total functions with German errors (`thiserror`) |
| `app` (+ `input`) | state machine: history, toggles, prompts, overlays, exam timer; consumes abstract `Input`s |
| `ui` | `matrix_view` (typesetting), `palette`, protocol, bottom bars, overlays |
| `tui`, `cli` | key mapping and event loop; argument parsing |

The crate is `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, denies `clippy::float_arithmetic` and, outside tests, `clippy::unwrap_used` / `clippy::expect_used`.

### Dependencies

| Crate | Why |
|-------|-----|
| `ratatui` | the TUI framework (it also provides the `crossterm` backend, used through its re-export, so there is no separate version to keep in sync) |
| `num-rational`, `num-bigint` | `BigRational`: exact fractions of unbounded size (`num-bigint` is pinned to 0.4 to match `num-rational`) |
| `num-traits` | `Zero`/`One`/`Signed` for the rational type |
| `thiserror` | error types with friendly messages |
| `proptest` *(dev)* | property tests |
| `insta` *(dev)* | snapshot tests of rendered screens |

Deliberately *not* used: `clap` (the option set is tiny; a hand-written, tested parser keeps the tree small) and `rand` (the exercise generator uses its own SplitMix64, so a given `--seed` produces the same matrix forever, independent of dependency upgrades).

## Testing

```sh
cargo test                                  # everything (about 195 tests)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

- **Unit tests** in every module, including parser tests with malformed input.
- **Property tests** (`tests/properties.rs`): every operation is undone by its inverse; row operations preserve the rank; the solver always ends in a valid Treppennormalform, which is unique across different random operation sequences. The rank is cross-checked against an independent oracle (largest nonvanishing minor) and the Treppennormalform against a literal transcription of its definition. Further properties cover parser totality, app history invariants and "rendering never panics at any terminal size". As a sanity check, deliberately breaking the solver makes three of these properties fail.
- **Snapshot tests** (`tests/snapshots.rs`, `insta`) of full screens on `TestBackend`, plus explicit colour assertions for the live analysis, both themes and the 256-colour fallback.
- **End-to-end tests** type the course example through the real key-handling path; **CLI tests** run the compiled binary.
- `tests/readme.rs` guarantees that the demo in this README is the real program output.

To update snapshots after an intended UI change: `INSTA_UPDATE=always cargo test --test snapshots`, review the diff, and paste `tests/snapshots/snapshots__readme_demo.snap` into this README.

## Design decisions

Decisions made on the user's behalf while building this, in case you want to change them:

- **Reset is reversible.** `reset` moves all steps to the redo stack instead of discarding them.
- **Pivot definition.** Only the *topmost* row whose first nonzero entry lies in a column owns that pivot. Entries below it in the same column are "still to become 0" (red) and not pivots — so in A₀ of the example you see one green pivot and two red entries, which is exactly the first step to take.
- **Adding a 0-fold is allowed** (a legal no-op); multiplying by 0 and adding or swapping a row with itself are rejected with an explanation.
- **Hints are computed from the current matrix**, not from A₀, by taking the first step of the reference solver. The solver prefers a pivot that is already 1 (swap before scaling), which avoids needless fractions.
- **Check mode** details the comparison with whichever reference (current matrix / Treppennormalform) is closer, and never reveals the entries of the Treppennormalform. It is disabled in exam mode, because it would answer "am I done?".
- **Exam mode** keeps your current matrix and starts the timer at 00:00; `new` restarts it. Even the tint of changed rows is switched off. Leaving exam mode reports the time, the number of steps and whether the Treppennormalform was reached.
- **Typography.** Matrices use the true minus (−); the Rechenprotokoll and messages use the ASCII `-` exactly as in the Maple wording and as you would type it.
- **Layout.** Minimum terminal size is 80×24. In the automatic layout stacked fractions are used whenever the matrix fits; otherwise inline fractions; otherwise the panel scrolls.
- **Presets adapt the size** when the requested size cannot satisfy them (e.g. `breit` needs more columns than rows) and say so.
- **The CLI's own `--help` text is in German** like the rest of the UI; this README is the English documentation.
- **Entries** of generated matrices are integers in −5…5; generated matrices are never the zero matrix and never already in Treppennormalform.

## License

MIT — see [LICENSE](LICENSE).
