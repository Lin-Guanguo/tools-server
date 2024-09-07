SHELL := /bin/bash

restart:
	kill -SIGUSR1 $$(pgrep -x tools-server.sh | head -n 1)

clippy:
	cargo clippy

port_check:
	sudo lsof -i :8001

echo:
	curl localhost:8001/echo?1=1 --data '111'

encrypt:
	curl localhost:8001/encrypt \
		-X GET \
		--data "$$(echo '{ "url": "http://localhost:8001/echo", "method": "POST", "headers": { "testh": "1" }, "body": "aGVsbG8=" }' | base64 -w 0 | rev)" | rev | base64 -d

