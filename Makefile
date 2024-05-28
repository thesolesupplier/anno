dev:
	cargo watch -x run

build:
	cargo lambda build --release

deploy:
	cargo lambda deploy --env-file .env --enable-function-url
