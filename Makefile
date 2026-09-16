.PHONY: help build package pkg rust-build release install install-package clean validate lint fmt fmt-check clippy test tests check audit deny machete changelog
.DEFAULT_GOAL := help
help:
	@echo "Available targets: make build, make check, make validate, make install"
lint:
	@shellcheck tools/sh/pkgbuild_local.sh
fmt:
	@cargo fmt --all
fmt-check:
	@cargo fmt --all -- --check
clippy:
	@cargo clippy --workspace --all-targets --all-features -- -D warnings
test:
	@cargo test --workspace --locked
tests: test
audit:
	@cargo audit
deny:
	@cargo deny check
machete:
	@cargo machete
check: lint fmt-check clippy test
rust-build:
	@cargo build --workspace --locked
release: check
	@cargo build --workspace --release --locked
package: check
	@tools/sh/pkgbuild_local.sh
pkg: package
build: package
install:
	@sudo pacman -U build/dist/*.zst --noconfirm --overwrite="*"
install-package: install
validate:
	@shellcheck tools/sh/pkgbuild_local.sh
	@cargo metadata --locked --no-deps --format-version 1 >/dev/null
	@cd packaging/arch/ci && makepkg -p PKGBUILD --printsrcinfo >/dev/null
	@cd packaging/arch/local && makepkg -p PKGBUILD --printsrcinfo >/dev/null
	@echo "Validation OK"
changelog:
	@git-cliff -o CHANGELOG.md
clean:
	@cargo clean
	@rm -rf build/
