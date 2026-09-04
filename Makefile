PREFIX ?= /usr
DESTDIR ?=

BIN_NAME := argvus-accounts
BIN := target/release/$(BIN_NAME)

.DEFAULT_GOAL := help

.PHONY: help build build-bin check lint fmt fmt-check validate validate-pkgbuild install uninstall reinstall clean

help:
	@echo "Available targets:"
	@echo "  make build"
	@echo "  make build-bin"
	@echo "  make check"
	@echo "  make fmt"
	@echo "  make fmt-check"
	@echo "  make validate"
	@echo "  make validate-pkgbuild"
	@echo "  make install"
	@echo "  make uninstall"
	@echo "  make clean"

build:
	@tools/build-local-package.sh

build-bin:
	cargo build --release --locked

check:
	cargo clippy --locked --all-targets --all-features -- -D warnings
	cargo test --locked

lint: check

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

validate: fmt-check check validate-pkgbuild

validate-pkgbuild:
	@if command -v makepkg >/dev/null 2>&1; then \
		cd packaging/arch && makepkg -p PKGBUILD --printsrcinfo >/dev/null; \
	else \
		echo "makepkg not found; skipping PKGBUILD syntax validation"; \
	fi

install: build-bin
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
	rm -rf dist
	rm -f packaging/arch/*.zst packaging/arch/*.tar.gz
