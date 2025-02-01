# Rocket 
uORocketry's rocket instrumentation system. 

## Setup and Building 

- Install Rust using downloader or script https://www.rust-lang.org/tools/install
- `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh`
- `cargo install cargo-make`
- `git clone https://github.com/uorocketry/argus.git`
- `cargo b`

## Documentation 
`cargo doc --open`

## Running code 
`cargo run --bin {board}`

## Tests 
- To run device tests `cargo make test-device` 
- - `cargo make test-temperature-board`
- To run host tests `cargo make test-host` 

## Helpful VSCode Extensions 
- probe-rs.probe-rs-debugger
- rust-lang.rust-analyzer
