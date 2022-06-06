[![](https://github.com/open-rmf/rmf_sandbox/workflows/style/badge.svg)](https://github.com/open-rmf/rmf_sandbox/actions/workflows/style.yaml)
[![](https://github.com/open-rmf/rmf_sandbox/workflows/ci_linux/badge.svg)](https://github.com/open-rmf/rmf_sandbox/actions/workflows/ci_linux.yaml)
[![](https://github.com/open-rmf/rmf_sandbox/workflows/ci_web/badge.svg)](https://github.com/open-rmf/rmf_sandbox/actions/workflows/ci_web.yaml)

# The RMF Sandbox

The RMF Sandbox is an experimental approach to visualizing large RMF deployment sites.
It is built in Rust using [Bevy](https://bevyengine.org/), an open-source Rust-based game engine.

Rust and Bevy allow The RMF Sandbox to target both desktop (Windows/Linux/Mac) and web (WebAssembly+WebGL/WebGPU) using the same codebase:
 * Web build: the browser sandbox provides maximum convenience, since there is nothing to build or install.
 * Desktop build: maximum performance, thanks to multithreading and lower-level GPU integration.

[Click here to use the web build in your browser](https://open-rmf.github.io/rmf_sandbox/).

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

Finally, we need some library packages:
```
sudo apt install libgtk-3-dev
```

# Install dependencies (Windows 11)

Make sure you install rust from the main rust website. Cargo should take care of the rest of the magic for you.

# Install extra dependencies for WebAssembly

These are only needed if you're going to build a WebAssembly binary:
```
sudo apt install binaryen
cargo install wasm-bindgen-cli basic-http-server
rustup target add wasm32-unknown-unknown
```

# Build and Run (Desktop)

Currently tested on Ubuntu 20.04.4 LTS and windows 11.

From the `rmf_sandbox` subdirectory:

```
cargo build
cargo run
```

# Build and Run (WebAssembly)

TODO: The web assembly version is highly experimental, currently it lacks important features like
saving/loading of map files.

```
scripts/build-web.sh
scripts/serve-web.sh
```

Then use your favorite web browser to visit `http://localhost:1234`
