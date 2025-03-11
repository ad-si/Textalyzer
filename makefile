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


.PHONY: release
release:
	@echo '1. `cai changelog <first-commit-hash>`'
	@echo '2. `git add ./changelog.md && git commit -m "Update changelog"`'
	@echo '3. `cargo release major / minor / patch`'
	@echo '4. Create a new GitHub release at https://github.com/ad-si/Textalyzer/releases/new'
	@echo \
		"5. Announce release on \n" \
		"   - https://x.com \n" \
		"   - https://bsky.app \n" \
		"   - https://this-week-in-rust.org \n" \
		"   - https://news.ycombinator.com \n" \
		"   - https://lobste.rs \n" \
		"   - Reddit \n" \
		"     - https://reddit.com/r/rust \n" \
		"     - https://reddit.com/r/ChatGPT \n" \
		"     - https://reddit.com/r/ArtificialInteligence \n" \
		"     - https://reddit.com/r/artificial \n"


.PHONY: clean
clean:
	rm -rf target
	rm -rf textalyzer-wasm/pkg
