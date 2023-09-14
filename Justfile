_help:
	@just --list --unsorted

# lint, build, install and tape
all: audit build (install './bin') tape render

# format, lint and check deps - requires `cargo-udeps` & `cargo-deny`
audit:
	cargo fmt
	cargo clippy -- -D clippy::all
	cargo update --dry-run
	cargo +nightly udeps
	cargo deny check bans advisories

# build in release - requires `cargo-limit`
build:
	cargo lbuild --release

# run in release - requires `cargo-limit`
run CMD:
	cargo lrun --release -- {{CMD}}

# run a command every time source files change - requires `cargo-watch`
dev CMD='just b':
	cargo watch -cs '{{CMD}}' -i 'res/*' -i 'bin/*'

# install the apps in a directory
install DIR='./bin':
	#!/usr/bin/env bash
	mkdir -p {{DIR}}
	cp target/release/aud {{DIR}}/aud

# create cli recordings - requires `vhs` & `parallel`
tape:
	#!/usr/bin/env bash
	parallel --ungroup vhs ::: vhs/*

# generate renders
render: 
	#!/usr/bin/env bash
	rm -f docs/renders.md
	for file in `ls vhs`; do
		f=${file%.*}
		echo "## \`$f\`"  >> docs/renders.md
		echo ""  >> docs/renders.md
		echo "![$f](../res/$f.gif)" >> docs/renders.md
		echo ""  >> docs/renders.md
	done
	

alias a := audit
alias b := build
alias d := dev
alias r := run
alias i := install
