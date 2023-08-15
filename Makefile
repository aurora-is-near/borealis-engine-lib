CARGO = cargo

release: check-compile
	$(CARGO) build --release

debug: check-compile
	$(CARGO) build

check: check-compile test check-format check-clippy

check-compile:
	$(CARGO) check --all-targets

test:
	$(CARGO) test --workspace --exclude aurora-refiner-app-integration-tests

integration-test:
	$(CARGO) test -p aurora-refiner-app-integration-tests

check-format:
	$(CARGO) fmt -- --check

check-clippy:
	$(CARGO) clippy -- -D warnings

format:
	$(CARGO) fmt

clean:
	$(CARGO) clean

.PHONY: release debug check check-compile test integration-test check-format check-clippy format clean

.SECONDARY:
.SUFFIXES:
