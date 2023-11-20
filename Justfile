_help:
	@just --list --unsorted

# lint, build, install and tape
all: audit (install '~/.aud/bin') tape

# format, lint and check deps - requires `cargo-deny`
audit:
	cargo fmt --all --check
	cargo clippy -- -D warnings -D clippy::all
	cargo update --dry-run
	cargo deny check bans advisories

# build in release - requires `cargo-limit`
build:
	cargo lbuild --release

# run in release - requires `cargo-limit`
run CMD:
	cargo lrun --release -- {{CMD}}

# run all tests - requires `cargo-nextest`
test:
	cargo nextest run --all-targets --all-features
	cargo test --doc

# run a command every time source files change - requires `cargo-watch`
dev CMD='just b':
	cargo watch -cs 'reset; {{CMD}}' -i 'res/*' -i 'out/*' -i 'lua/api/examples/*'

# run the benchmarks
bench:
	cargo bench --features bench

# clean-build a release build with a timing report
buildtime:
	cargo clean && cargo build --timings --verbose --release

# install the apps in a directory
install DIR='./out': build
	#!/usr/bin/env bash
	mkdir -p {{DIR}}
	cp target/release/aud {{DIR}}/aud

_tape CMD:
    vhs res/vhs/{{CMD}}.tape

# create CLI recordings - requires `vhs` & `parallel`
tape:
	#!/usr/bin/env bash
	parallel --ungroup vhs ::: res/vhs/*

# tail a log file - request `bat`
log FILE='./out/aud.log':
	# log highlighting is available but yaml looks nicer
	tail -n5 -f {{FILE}} | bat --paging=never -l=yaml --style=plain

# log localhost udp comms
udpdump:
	sudo tcpdump -i lo0 udp port 8080 -v # -X

# run-once setup your development environment for this project
setup: (_setup_packages)
	cargo install cargo-deny cargo-watch cargo-nextest bat
	echo "#!/bin/sh\n\n"\
	"just audit\n"\
	> .git/hooks/pre-push
	chmod +x .git/hooks/pre-push

# check for unused dependencies - requires cargo-udeps
_udeps:
	cargo +nightly udeps

[linux]
_setup_packages:
	sudo apt-get install parallel vhs pkg-config lua

[macos]
_setup_packages:
	brew install parallel vhs pkg-config lua

alias t := test
alias i := install
alias b := build
alias d := dev
alias r := run
