dev:
	cargo watch -x "run --bin api" --no-vcs-ignores

build:
	cargo lambda build --release --bin api

deploy:
	cargo lambda deploy --enable-function-url --env-file .env.prod --binary-name api

release: build deploy