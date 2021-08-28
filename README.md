# rusty-jam-fluidish
Rusty game jam submission. It's not yet a game, but we had fun.
Uses shipyard and macroquad.

Runs stand-alone:

`cargo run`

or as a wasm executable:

```
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/grid_world.wasm public
microserver public
```

then point your browser at localhost:9090

