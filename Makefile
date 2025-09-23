RUST_RS_FILES := $(shell find . -type f -name '*.rs' 2>/dev/null | LC_ALL=C sort)
RUST_SOURCES := Cargo.toml Cargo.lock $(RUST_RS_FILES)
WASM_OUT := ts/gen/redb-opfs_bg.wasm
DTS_OUT := ts/gen/redb-opfs.d.ts
JS_OUT := ts/gen/redb-opfs.js

# If RELEASE is nonempty, build in release mode
# Otherwise build in dev mode, which is much faster
WASM_BUILD_ARGS := $(if $(RELEASE),,--dev)

.PHONY: clean clean-bun clean-pack clean-wwex
clean:
	cargo clean
	$(MAKE) clean-bun
	$(MAKE) clean-pack
	$(MAKE) clean-wwex
	$(MAKE) clean-riww

clean-bun:
	rm -rf $(WWEX)/node_modules

clean-pack:
	rm -rf ts/gen

clean-wwex:
	rm -rf $(WWTARGET)

clean-riww:
	cd examples/rust-in-web-worker && cargo clean

Cargo.lock: Cargo.toml
	cargo check
	@touch $@

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

WWEX := examples/web-worker
WWEX_HTML := $(WWEX)/src/index.html
WWEX_TS := $(shell find $(WWEX)/src -type f -name '*.ts' 2>/dev/null | LC_ALL=C sort)
WWEX_SOURCES := $(WWEX)/bun.lock $(WWEX)/package.json $(WWEX_TS)
WWTARGET := target/web-worker-example
BUNDLE_INDEX := $(WWTARGET)/index.js
BUNDLE_WORKER := $(WWTARGET)/worker.js
BUNDLE_WASM := $(WWTARGET)/redb-opfs_bg.wasm

$(BUNDLE_INDEX) $(BUNDLE_WORKER) $(BUNDLE_WASM) &: $(WWEX_HTML) $(WASM_OUT) $(JS_OUT) $(WWEX_SOURCES)
	cd examples/web-worker/src && \
	bun build \
	--target browser \
	--format esm \
		index.ts \
		worker.ts \
	--outdir ../../../$(WWTARGET)
	cp $(WWEX_HTML) $(WASM_OUT) $(WWTARGET)


.PHONY: web-worker-example
web-worker-example: $(BUNDLE_INDEX) $(BUNDLE_WORKER) $(BUNDLE_WASM)
#	cargo install --locked miniserve
	miniserve \
		--index index.html \
		--port 8000 \
	$(WWTARGET)

RIWW_RS_FILES := $(shell find examples/rust-in-web-worker -type f -name '*.rs' 2>/dev/null | LC_ALL=C sort)
RIWW_SOURCES := examples/rust-in-web-worker/Cargo.toml examples/rust-in-web-worker/Cargo.lock $(RIWW_RS_FILES)
RIWW_WASM_OUT := ts/gen/riww_bg.wasm
RIWW_DTS_OUT := ts/gen/riww.d.ts
RIWW_JS_OUT := ts/gen/riww.js

$(RIWW_JS_OUT) $(RIWW_DTS_OUT) $(RIWW_WASM_OUT) &: $(RIWW_SOURCES)
	cd examples/rust-in-web-worker && \
	wasm-pack build \
		--locked \
		--no-pack \
		--out-dir ../../ts/gen \
		--out-name riww \
		--mode normal \
		--target web \
		$(WASM_BUILD_ARGS)
