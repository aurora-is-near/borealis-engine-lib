CARGO = cargo
MAKE = make

release: check-compile
	cd aurora-standalone && $(CARGO) build --release

debug: check-compile
	cd aurora-standalone && $(CARGO) build

check: check-compile test check-format check-clippy

check-compile:
	cd aurora-standalone && $(CARGO) check --all-targets

test:
	cd aurora-standalone && $(CARGO) test

check-format:
	cd aurora-standalone && $(CARGO) fmt -- --check

check-clippy:
	cd aurora-standalone && $(CARGO) clippy -- -D warnings

format:
	cd aurora-standalone && $(CARGO) fmt

clean:
	cd aurora-standalone && $(CARGO) clean

.PHONY: release debug check check-compile test check-format check-clippy format clean

.SECONDARY:
.SUFFIXES:
