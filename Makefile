RUST_RS_FILES := $(shell find . -type f -name '*.rs' 2>/dev/null | LC_ALL=C sort)
RUST_SOURCES := Cargo.toml Cargo.lock $(RUST_RS_FILES)
WASM_OUT := ts/gen/redb-opfs_bg.wasm
DTS_OUT := ts/gen/redb-opfs.d.ts
JS_OUT := ts/gen/redb-opfs.js

# If RELEASE is nonempty, build in release mode
# Otherwise build in dev mode, which is much faster
WASM_BUILD_ARGS := $(if $(RELEASE),,--dev)

Cargo.lock: Cargo.toml
	cargo check

$(JS_OUT) $(DTS_OUT) $(WASM_OUT) &: $(RUST_SOURCES)
	wasm-pack build \
		--locked \
		--no-pack \
		--out-dir ts/gen \
		--out-name redb-opfs \
		--mode normal \
		--target web \
		$(WASM_BUILD_ARGS)

# human name for building wasm
.PHONY: wasm-build
wasm-build: $(WASM_OUT)

.PHONY: web-worker-example
web-worker-example: $(WASM_OUT)
	cd examples/web-worker && \
	timeout 1s bun run src/index.ts
