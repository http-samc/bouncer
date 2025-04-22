clean:
	rm -rf target

build:
	cargo build --release

run:
	cargo run --release

dev:
	cargo build --release && cargo run --release -- --config examples/default/bouncer.config.yaml

fix:
	cargo fmt
	cargo clippy --fix --allow-dirty --allow-staged
	cargo fix --allow-dirty --allow-staged --lib -p bouncer
