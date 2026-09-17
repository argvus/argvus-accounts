.PHONY: help build package pkg rust-build release install install-package clean validate lint lint-shell fmt fmt-check clippy test tests check audit deny machete changelog
.DEFAULT_GOAL := help
help:
	@echo "Available targets: make build, make check, make validate, make install"
lint-shell:
	@for root in tools packaging/arch/common src; do \
		if [ -d "$$root" ]; then \
			find "$$root" -type f -name '*.sh' -exec shellcheck -e SC1090 -e SC2034 -e SC2154 {} +; \
		fi; \
	done
	@for root in tools packaging/arch/common src; do \
		if [ -d "$$root" ]; then \
			find "$$root" -type f -name '*.sh' -exec bash -n {} +; \
		fi; \
	done
	@git diff --check
	@echo "Lint Shell OK"

lint: lint-shell
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
