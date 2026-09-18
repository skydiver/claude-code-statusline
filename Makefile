.PHONY: all clean dist dev test preview help

DIST := dist
BIN := ccline

# Bare `make` lists the targets rather than building — `make dist` builds.
# Help is generated from the `##` comments below, so it can never drift out
# of sync with the targets it documents.
.DEFAULT_GOAL := help

help: ## List the available targets
	@echo "ccline — available targets:"
	@echo
	@grep -hE '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN { FS = ":.*?## " } { printf "  \033[36m%-8s\033[0m %s\n", $$1, $$2 }'
	@echo

all: dist ## Alias for `make dist`

dist: ## Build the release binary and copy it into dist/
	cargo build --release
	@mkdir -p $(DIST)
	cp target/release/$(BIN) $(DIST)/$(BIN)
	@echo "\n✅ Build complete. Output:"
	@ls -lh $(DIST)/$(BIN)
	@echo "\nRun:  ./dist/$(BIN)"

dev: ## Run via cargo (prints a usage banner without piped stdin)
	cargo run

test: ## Run the full unit suite
	cargo test

preview: ## Render the statusline across a scenario matrix, in real color
	cargo build --release
	@python3 scripts/preview.py

clean: ## Remove build artifacts (cargo clean plus dist/)
	cargo clean
	rm -rf $(DIST)
