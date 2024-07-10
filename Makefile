dev:
	cargo watch -x run --no-vcs-ignores

build:
	cargo lambda build --release

deploy:
	cargo lambda deploy --enable-function-url --env-file .env.prod

release: build deploy