.PHONY: engine test wasm web build-web

engine:
	cargo build --release -p rookery-engine

test:
	cargo test --release -p rookery-engine

wasm:
	wasm-pack build wasm --release --target web --out-dir ../web/src/wasm-bindings

web: wasm
	cd web && npm install && npm run dev

build-web: wasm
	cd web && npm install && npm run build
