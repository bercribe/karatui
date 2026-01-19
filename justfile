# https://just.systems

build:
    cargo build

test:
    cargo test

fmt:
    cargo fmt

lint:
    cargo clippy

check-all: build test fmt lint
