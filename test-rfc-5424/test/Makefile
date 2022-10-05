.PHONY: check

check:
	rm -f /tmp/syslog.tmp
	./test-server start
	./test-server run-tests
	./test-server get-syslog /tmp/syslog.tmp
	./check-syslog-tracing /tmp/syslog.tmp && ./test-server stop
