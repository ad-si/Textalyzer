.PHONY: help
help: makefile
	@tail -n +4 makefile | grep ".PHONY"


textalyzer-wasm/pkg: textalyzer-wasm/src/lib.rs textalyzer-wasm/Cargo.toml
	if command -v wasm-pack >/dev/null 2>&1; \
	then \
		cd textalyzer-wasm \
		&& wasm-pack build --target web; \
	else \
		echo cargo binstall wasm-pack; \
	fi


.PHONY: build
build: textalyzer-wasm/pkg


.PHONY: fmt
fmt:
	cargo fmt


.PHONY: test
test:
	cargo test
	cargo clippy


.PHONY: install
install:
	cargo install --path textalyzer


.PHONY: clean
clean:
	rm -rf target
	rm -rf textalyzer-wasm/pkg
