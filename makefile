.PHONY: help
help: makefile
	@tail -n +4 makefile | grep ".PHONY"


textalyzer-wasm/pkg: textalyzer-wasm/src/lib.rs textalyzer-wasm/Cargo.toml
	cd textalyzer-wasm && wasm-pack build --target web


.PHONY: build
build: textalyzer-wasm/pkg


.PHONY: test
test:
	cargo test
	cargo clippy


.PHONY: install
install:
	cargo install --path textalyzer
