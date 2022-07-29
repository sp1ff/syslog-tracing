.PHONY: check

check:
	rm -f /tmp/syslog.tmp
	./test-server start
	cargo test --features=rsyslogd
	./test-server get-syslog /tmp/syslog.tmp
	./check-syslog-tracing /tmp/syslog.tmp && ./test-server stop
