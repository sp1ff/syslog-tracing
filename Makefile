.PHONY: all clean check doc tags

all: tags doc
	cargo build

clean:
	cargo clean
	rm -f TAGS tags

check:
	cargo test
	$(MAKE) -C test-rfc-5424 static
	docker build --no-cache -t tracing-syslog-test-server:latest -f test-rsyslogd.docker .
	$(MAKE) -C test-rfc-5424 check

doc:
	cargo doc

tags:
	rusty-tags -O TAGS emacs
	rusty-tags -O tags vi

