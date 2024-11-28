dev:
	cargo watch -x "run --bin api" --no-vcs-ignores

build:
	cargo lambda build --release --bin api

deploy:
	cargo lambda deploy --binary-name api anno --enable-function-url --env-file .env.prod

release: build deploy