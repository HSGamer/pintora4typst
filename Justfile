# Justfile for rust-pintora plugin

export PATH := env_var("HOME") + "/.cargo/bin:" + env_var("PATH")

# Default recipe: build and optimize the WASM plugin
build:
	cargo build --release --target wasm32-wasip1
	wasi-stub -r 0 ./target/wasm32-wasip1/release/rust_pintora.wasm -o typst-package/pintora.wasm
	wasm-opt typst-package/pintora.wasm -O3 --converge --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext -o typst-package/pintora.wasm

# Build the WASM plugin without the slow wasm-opt pass (useful for rapid development)
dev:
	cargo build --target wasm32-wasip1
	wasi-stub -r 0 ./target/wasm32-wasip1/debug/rust_pintora.wasm -o typst-package/pintora.wasm

# Compile the test documents in the `tests/` directory to verify the plugin works
test: build
	time typst compile --root . tests/test.typ tests/test.pdf

# Clean build artifacts
clean:
	cargo clean
	rm -f typst-package/pintora.wasm tests/*.pdf
