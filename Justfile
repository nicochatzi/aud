_help:
	@just --list --unsorted

# format and lint
check:
	cargo fmt && cargo clippy -- -D clippy::all

# build in release
build:
	cargo build --release

# dev with cargo-watch + cargo-limit
dev:
	cargo watch -cx 'lbuild --release'

# install the apps in a directory, defaults in ${ROOT_OF_PROJECT}/bin/
install INSTALL_DIR="./bin":
	#!/usr/bin/env bash
	mkdir -p {{INSTALL_DIR}}
	for app in `ls src/apps`; do
		cp target/release/$app {{INSTALL_DIR}}/$app
	done
