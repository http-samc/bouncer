clean:
	rm -rf target

build:
	cargo build --release

run:
	env $(shell cat .env) cargo run --release -- --config examples/default/bouncer.config.yaml

dev:
	make build && make run

fix:
	cargo fmt
	cargo clippy --fix --allow-dirty --allow-staged
	cargo fix --allow-dirty --allow-staged --lib -p bouncer
