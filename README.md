# The RMF Sandbox

The RMF Sandbox is an experimental approach to visualizing large RMF deployment sites.
It will be built in Rust using [Bevy](https://bevyengine.org/), an open-source Rust-based game engine.

Rust and Bevy will allow The RMF Sandbox to target both desktop (Windows/Linux/Mac) and web (WebAssembly+WebGL/WebGPU) using the same codebase.
For example, the [Traffic Editor III](https://github.com/open-rmf/traffic_editor_iii) experiment can be [used in a web browser](https://open-rmf.github.io/traffic_editor_iii) for convenience, or it can compiled to a native executable for maximum performance.

# Helpful Links

 * [Bevy Engine](https://bevyengine.org/)
 * [Bevy Cheat Book](https://bevy-cheatbook.github.io/)
 * [Rust Book](https://doc.rust-lang.org/stable/book/)

# Install dependencies (Ubuntu 20.04)

We need a newer Rust than what comes with Ubuntu 20.04.

First make sure you don't have any distro-installed Rust stuff on your machine:
```
sudo apt remove rustc cargo
```

If you don't have it already, install `rustup` from the Rust website: https://www.rust-lang.org/tools/install
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Just select the normal defaults (option 1).
A bunch of stuff will happen. Be sure to close and re-open your terminal afterwards, so that it gets all the new stuff.

Alternatively, if you already have a Rust installation managed by `rustup`, you can just do this to bring it up-to-date: `rustup update`

Now install the necessary tooling for WebAssembly:
```
sudo apt install binaryen
cargo install wasm-bindgen-cli basic-http-server
```

# Build and Run (Desktop)

### Desktop build (tested on Ubuntu 20.04)

From the `rmf_sandbox` subdirectory:

```
cargo build
cargo run
```

# Build and Run (WebAssembly)

(currently broken, it's a work-in-progress)

```
scripts/build-web.sh
scripts/serve-web.sh
```

Then use your favorite web browser to visit `http://localhost:1234`
