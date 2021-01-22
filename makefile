build:
	cargo +nightly-2020-10-05 build --release

dev:
	./target/release/subzero \
	--dev \
	--base-path ./data/dev \
	--name alphaville \
	--port 30333 \
	--ws-port 9944 \
	--rpc-port 9933 \
	--rpc-methods unsafe \

purge-dev:
	./target/release/subzero purge-chain -y --dev
