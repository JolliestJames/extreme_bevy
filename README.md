# Extreme Bevy

Exploring  implementation of a peer-to-peer multiplayer game in Bevy. Following along [here]().

### Play Extreme Bevy

Clone the repo and make sure you've got Rust and the following dependencies installed.
```
rustup target install wasm32-unknown-unknown
cargo install wasm-server-runner
cargo install cargo-watch
cargo install matchbox_server
```

Then run the following commands in separate terminals:

```
cargo watch -cx run
```

```
matchbox_server
```

