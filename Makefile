
test_echo:
	curl localhost:8001/echo?1=1 --data '111'

test_encrypt:
	curl localhost:8001/encrypt \
		-X POST \
		--data "$$(echo '{ "url": "http://localhost:8001/echo", "method": "POST", "headers": { "testh": "1" }, "body": "124121211211" }' | base64 -w 0 | rev)"
