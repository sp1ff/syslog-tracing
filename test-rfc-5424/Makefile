.PHONY: all static check

all:
	cargo build

static:
	cargo build --target=x86_64-unknown-linux-musl

check:
	cargo test
	$(MAKE) -C test check

