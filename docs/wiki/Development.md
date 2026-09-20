# Development

## Main commands

```bash
cd app
flutter pub get
flutter analyze
flutter test
```

```bash
cd core
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

## Bridge generation

```bash
cd app
flutter_rust_bridge_codegen generate --config-file flutter_rust_bridge.yaml
```

## Android

Use Java 17 and Android NDK 28.2.13676358. The release workflow is the reference for cross-ABI compiler environment variables.

Keep generated build output out of source control.
