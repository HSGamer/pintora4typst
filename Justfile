# Justfile for rust-pintora plugin
PINTORA_VERSION := "0.2.1"

export PATH := env_var("HOME") + "/.cargo/bin:" + env_var("PATH")

# Default recipe: build and optimize the WASM plugin
build: setup
	cargo build --release --target wasm32-wasip1
	wasi-stub -r 0 ./target/wasm32-wasip1/release/rust_pintora.wasm -o typst-package/pintora.wasm
	wasm-opt typst-package/pintora.wasm -O3 --converge --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --strip-debug --strip-producers --strip-target-features -o typst-package/pintora.wasm

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

name          := `grep '^name' typst-package/typst.toml | cut -d'"' -f2`
version       := `grep '^version' typst-package/typst.toml | cut -d'"' -f2`
dist_dir      := "typst-package"
packages_fork := "git@github.com:HSGamer/typst-packages.git"
packages_dir  := "typst-packages"

# Publish to typst/packages: sparse-clone fork, copy dist, commit and push
publish: build
    #!/usr/bin/env bash
    set -euo pipefail
    BRANCH="packages/{{ name }}/{{ version }}"
    PKG_PATH="packages/preview/{{ name }}/{{ version }}"

    echo "Cloning {{ packages_fork }} (sparse)..."
    rm -rf "{{ packages_dir }}"
    git clone --depth 1 --sparse --filter=blob:none "{{ packages_fork }}" "{{ packages_dir }}"

    cd "{{ packages_dir }}"
    # Delete remote branch if it exists from a previous attempt
    git push origin --delete "$BRANCH" 2>/dev/null || true
    git checkout -b "$BRANCH"
    git sparse-checkout set "$PKG_PATH"

    echo "Copying {{ dist_dir }} → $PKG_PATH..."
    mkdir -p "$PKG_PATH"
    cp -r "../{{ dist_dir }}/." "$PKG_PATH/"
    cp "../README.md" "$PKG_PATH/README.md"
    cp "../LICENSE" "$PKG_PATH/LICENSE"

    git add "$PKG_PATH"
    git commit -m "{{ name }}:{{ version }}"
    git push -u origin "$BRANCH"

    cd ..
    rm -rf "{{ packages_dir }}"
    echo "Done! Branch '$BRANCH' pushed to {{ packages_fork }}"
    echo "Open a PR at https://github.com/typst/packages to publish."