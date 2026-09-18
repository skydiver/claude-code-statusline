.PHONY: all clean dist dev test preview

DIST := dist
BIN := ccline

all: dist

# Build release binary and collect into dist/
dist:
	cargo build --release
	@mkdir -p $(DIST)
	cp target/release/$(BIN) $(DIST)/$(BIN)
	@echo "\n✅ Build complete. Output:"
	@ls -lh $(DIST)/$(BIN)
	@echo "\nRun:  ./dist/$(BIN)"

# Run in dev mode
dev:
	cargo run

# Run all tests
test:
	cargo test

# Render the statusline across a scenario matrix, in real color
preview:
	cargo build --release
	@python3 scripts/preview.py

# Remove build artifacts
clean:
	cargo clean
	rm -rf $(DIST)
