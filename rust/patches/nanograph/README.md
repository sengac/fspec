# nanograph

[![CI](https://github.com/nanograph/nanograph/actions/workflows/ci.yml/badge.svg)](https://github.com/nanograph/nanograph/actions/workflows/ci.yml)
[![Rust: stable](https://img.shields.io/badge/rust-1.94.1%20stable-orange?logo=rust)](rust-toolchain.toml)
[![Crates.io](https://img.shields.io/crates/v/nanograph)](https://crates.io/crates/nanograph)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

On-device typed property graph DB for agents and humans. One CLI. One folder. Schema-as-code. No server.

Built on Rust, Lance, Arrow, and DataFusion.

**[Website](https://nanograph.io)** | **[Docs](https://nanograph.io/docs)** | **[Examples](examples/starwars/)**

## Why nanograph

- **On-device** -- no server, no cloud, no Docker. Everything stays on your machine.
- **Schema-as-code** -- `.pg` files version-controlled in git, enforced at query time. No "property not found" at runtime.
- **Built for agents** -- Claude reads, writes, and traverses the graph natively.
- **Fast** -- Rust + Lance + Arrow columnar execution. Sub-millisecond opens, ACID, time-travel.
- **Full search stack** -- full-text, semantic, fuzzy, BM25, hybrid, graph-constrained reranking.
- **Built-in CDC** -- every mutation logged to a ledger. Replay, audit, sync from any version.
- **Zero setup** -- create a graph and start querying. Delete and recreate in seconds.

## Install

```bash
brew install nanograph/tap/nanograph
```

Or from source (requires [Rust](https://www.rust-lang.org/tools/install) `1.94.1` and `protoc`):

```bash
cargo install nanograph-cli --locked
```

### SDKs

- **TypeScript/Node.js** -- `npm install nanograph-db`
- **Swift** -- Swift Package via the [nanograph-ffi](crates/nanograph-ffi/swift/) wrapper

## Quick start

```bash
cd examples/starwars

nanograph init
nanograph load --data starwars.jsonl --mode overwrite
nanograph lint --query starwars.gq
nanograph run search "father and son conflict"
```

For new projects, `nanograph init` scaffolds `nanograph.toml` for shared defaults and `.env.nano` for local secrets. See [`docs/user/config.md`](docs/user/config.md) for config, env, and query alias details.

See [`examples/starwars/`](examples/starwars/) and [`examples/revops/`](examples/revops/) for the canonical ready-to-run examples.

## Use cases

- Context graphs and decision traces for AI agents
- Agentic memory with typed, sub-100ms local queries
- Personal knowledge graphs with schema enforcement
- Dependency and lineage modeling
- Feature generation for ML pipelines

## License

MIT
