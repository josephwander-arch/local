# Contributing to local MCP Server

Thank you for your interest in contributing.

## Reporting Issues

Open an issue at [github.com/josephwander-arch/local-mcp/issues](https://github.com/josephwander-arch/local-mcp/issues).

Include:
- Windows version and architecture (x64 / ARM64)
- local server version (`server_health` output)
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs from `C:\CPC\logs\local_activity.jsonl`

## Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Run the test suite: `cargo test`
5. Run clippy: `cargo clippy -- -D warnings`
6. Commit with a clear message describing the change
7. Push and open a PR against `main`

### PR Guidelines

- One logical change per PR
- Include a summary of what changed and why
- If adding a new tool, include a skill reference entry and an example
- If modifying breadcrumb behavior, update the hooks and examples accordingly

## Building from Source

### Prerequisites

- Rust 1.75+ with the `stable` toolchain
- Windows 10/11

### Build

```bash
# x64
cargo build --release --target x86_64-pc-windows-msvc

# ARM64
cargo build --release --target aarch64-pc-windows-msvc
```

The binary will be at `target/<target>/release/local.exe`.

### Cross-Compile (ARM64 from x64)

```bash
rustup target add aarch64-pc-windows-msvc
cargo build --release --target aarch64-pc-windows-msvc
```

## Code of Conduct

Be respectful. Focus on the work. We're here to build useful tools.

## License

By contributing, you agree that your contributions will be licensed under
the Apache License 2.0.
