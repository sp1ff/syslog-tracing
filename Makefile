.PHONY: all clean check doc

all:
	cargo build && cargo doc

clean:
	cargo clean

check:
	cargo test
	$(MAKE) -C test

doc:
	cargo doc

