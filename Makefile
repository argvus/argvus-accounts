PREFIX ?= /usr
DESTDIR ?=

BIN_NAME := argvus-accounts
BIN := target/release/$(BIN_NAME)

.PHONY: build check lint fmt fmt-check install uninstall reinstall clean

build:
	cargo build --release --locked

check:
	cargo clippy --locked --all-targets --all-features -- -D warnings
	cargo test --locked

lint: check

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

install: build
	install -Dm755 "$(BIN)" "$(DESTDIR)$(PREFIX)/bin/$(BIN_NAME)"
	install -Dm644 README.md "$(DESTDIR)$(PREFIX)/share/doc/$(BIN_NAME)/README.md"
	install -Dm644 LICENSE "$(DESTDIR)$(PREFIX)/share/licenses/$(BIN_NAME)/LICENSE"

uninstall:
	rm -f "$(DESTDIR)$(PREFIX)/bin/$(BIN_NAME)"
	rm -rf "$(DESTDIR)$(PREFIX)/share/doc/$(BIN_NAME)"
	rm -rf "$(DESTDIR)$(PREFIX)/share/licenses/$(BIN_NAME)"

reinstall: uninstall install

clean:
	cargo clean
