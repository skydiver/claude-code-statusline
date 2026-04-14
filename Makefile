.PHONY: all clean dist dev test

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

# Remove build artifacts
clean:
	cargo clean
	rm -rf $(DIST)
