# Justfile for rust-pintora plugin
PINTORA_VERSION := "0.2.1"

export PATH := env_var("HOME") + "/.cargo/bin:" + env_var("PATH")

# Default recipe: build and optimize the WASM plugin
build: setup
	cargo build --release --target wasm32-wasip1
	wasi-stub -r 0 ./target/wasm32-wasip1/release/rust_pintora.wasm -o typst-package/pintora.wasm
	wasm-opt typst-package/pintora.wasm -O3 --converge --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext -o typst-package/pintora.wasm

# Build the WASM plugin without the slow wasm-opt pass (useful for rapid development)
dev: setup
	cargo build --target wasm32-wasip1
	wasi-stub -r 0 ./target/wasm32-wasip1/debug/rust_pintora.wasm -o typst-package/pintora.wasm

setup:
	@if [ ! -f js/runtime.esm.js ]; then \
		echo "js/runtime.esm.js not found, setting up..."; \
		npm pack @pintora/target-wintercg@{{PINTORA_VERSION}}; \
		mkdir -p js; \
		tar -xzf pintora-target-wintercg-{{PINTORA_VERSION}}.tgz --strip-components=2 package/dist/runtime.esm.js; \
		mv runtime.esm.js js/runtime.esm.js; \
		rm pintora-target-wintercg-{{PINTORA_VERSION}}.tgz; \
	fi

# Compile the test documents in the `tests/` directory to verify the plugin works
test: build
	time typst compile --root . tests/test.typ tests/test.pdf

# Clean build artifacts
clean:
	cargo clean
	rm -f typst-package/pintora.wasm tests/*.pdf
