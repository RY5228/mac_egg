# CellE Rust (egg) Implementation

This repository contains the Rust part of the paper **CellE: Automated Standard Cell Library Extension via Equality Saturation**. It uses [`egg`](https://github.com/egraphs-good/egg) for equality saturation, targeting standard-cell netlist rewriting and frequent subcircuit mining.

- Paper: <https://arxiv.org/abs/2603.12797>
- Title: CellE: Automated Standard Cell Library Extension via Equality Saturation

## Features

- Read standard-cell netlists (Verilog) and technology libraries (Liberty)
- Apply rewrite rules (JSON) on e-graphs
- Serialize rewritten e-graphs to JSON
- Mine frequent subcircuits on e-graphs (GSpan)
- Export candidate fused cells as `.blif`

## Requirements

- Rust toolchain (latest stable is recommended)
- Linux/macOS (some test/visualization paths are Linux-oriented)

## Build

```bash
cargo build
```

## CLI Usage

```bash
cargo run -- \
  --input <netlist.v> \
  --library <library.lib> \
  --output <output_dir> \
  --rules <rule1.json> \
  --rules <rule2.json> \
  --min-support 10 \
  --max-size 5 \
  --max-num-inputs 3 \
  --top-k 10
```

You can also skip the "netlist + rewriting" stage and mine directly from a serialized e-graph:

```bash
cargo run -- \
  --input-egraph <rewritten_egraph.json> \
  --library <library.lib> \
  --output <output_dir> \
  --top-k 10
```

To see all available options:

```bash
cargo run -- --help
```

## Quick Example (Using Repo Test Data)

```bash
cargo run -- \
  --input test/add2_map_abc.v \
  --library test/asap7sc6t_SELECT_LVT_TT_nldm.lib \
  --output output \
  --rules rules/6t.json \
  --rules rules/6t_comm_rules.json \
  --rules rules/6t_dmg_rules.json \
  --rules rules/6t_inv_rules.json \
  --top-k 10 \
  -v
```

## Inputs and Outputs

Inputs:
- `--input`: standard-cell netlist (`.v`)
- `--input-egraph`: serialized e-graph (`.json`), mutually exclusive with `--input`
- `--library`: Liberty file (`.lib`)
- `--rules`: rewrite rule files (`.json`, repeatable)
- `--cell-area`: optional cell-area JSON (for area-based ranking)

Outputs (under `--output` directory):
- `rewritten_egraph.json` (or the path set by `--output-egraph`)
- `FUSED_CELL_*.blif` (frequent subcircuit candidates)

## Project Structure

- `src/main.rs`: CLI entry point, orchestrates rewriting and mining
- `src/rule.rs`: rule loading and conversion
- `src/io/`: Verilog / Liberty / AIGER / Bench I/O
- `src/mining.rs`: frequent subcircuit mining
- `src/language.rs`: e-graph language definition
- `extraction-gym/`: extraction-related submodule

## Citation

If you use this implementation in research or engineering work, please cite the CellE paper:

```text
CellE: Automated Standard Cell Library Extension via Equality Saturation
https://arxiv.org/abs/2603.12797
```
