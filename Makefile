.PHONY: all clean check doc

all: tags doc
	cargo build

clean:
	cargo clean

check:
	cargo test
	$(MAKE) -C test-rfc-5424 static
	docker build --no-cache -t tracing-syslog-test-server:latest -f test-rsyslogd.docker .
	$(MAKE) -C test-rfc-5424 check

doc:
	cargo doc
