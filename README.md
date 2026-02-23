# Rust-PubSub-Client

Rust implementation of a pub/sub client built on tokio, rust_socketio, and dashmap. Includes Criterion benchmarks for performance measurement.

## Stack

[![Stack](https://skillicons.dev/icons?i=rust&theme=dark)](https://skillicons.dev)
## Structure

- `Cargo.toml` -- Project manifest and dependencies
- `src/` -- Library source
  - `lib.rs` -- Crate root
  - `client.rs` -- Pub/sub client implementation
  - `config.rs` -- Configuration handling
  - `message.rs` -- Message types
  - `error.rs` -- Error definitions
  - `idempotence.rs` -- Idempotent message processing
- `benches/client_benchmark.rs` -- Criterion benchmarks
- `examples/` -- Usage examples
  - `simple_client.rs` -- Basic client usage
  - `wildcard_handler.rs` -- Wildcard subscription handling
- `tests/` -- Integration and unit tests
- `PERFORMANCE.md` -- Performance notes and benchmark results
