
build:
	cargo build --release

dev:
	./target/release/subzero-node \
	--dev \
	--base-path ./data/dev \
	--name alphaville \
	--port 30333 \
	--ws-port 9944 \
	--rpc-port 9933 \
	--rpc-methods unsafe \

purge-dev:
	./target/release/subzero-node purge-chain -y --dev

reset-build:
	rm -rf ./target

#
# custom local testnet
# specs/customLocalSpecRaw
#

zva:
	./target/release/subzero \
	--validator \
	--chain ./specs/zero_alphaville.json \
	--base-path ./data/za \
	--name neuromancer \
	--port 30333 \
	--ws-port 9944 \
	--rpc-port 9933 \
	--rpc-methods unsafe

zvb:
	./target/release/subzero \
	--validator \
	--chain ./specs/zero_alphaville.json \
	--base-path ./data/zb \
	--name count-zero \
	--bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWMnmDMUeB1WwwEMM7qPZTn7sxSFHCEjYjasoS6qkaLYbq \
	--port 30335 \
	--ws-port 9946 \
	--rpc-port 9935 \
	--rpc-methods unsafe


zvc:
	./target/release/subzero \
	--validator \
	--chain ./specs/zero_alphaville.json \
	--base-path ./data/zc \
	--name monalisa-overdrive \
	--bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWMnmDMUeB1WwwEMM7qPZTn7sxSFHCEjYjasoS6qkaLYbq \
	--port 30336 \
	--ws-port 9947 \
	--rpc-port 9936 \
	--rpc-methods unsafe

zvd:
	./target/release/subzero \
	--validator \
	--chain ./specs/zero_alphaville.json \
	--base-path ./data/zd \
	--name burning-chrome \
	--bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWMnmDMUeB1WwwEMM7qPZTn7sxSFHCEjYjasoS6qkaLYbq \
	--port 30337 \
	--ws-port 9948 \
	--rpc-port 9937 \
	--rpc-methods unsafe

#
# preset local testnet
#

subzero-a:
	./target/release/subzero-node \
	--validator \
	--chain local \
	--base-path ./data/a \
	--name neuromancer \
	--port 30333 \
	--ws-port 9944 \
	--rpc-port 9933 \
	--rpc-methods unsafe \
	--node-key 0000000000000000000000000000000000000000000000000000000000000001

subzero-b:
	./target/release/subzero-node \
	--validator \
	--chain local \
	--base-path ./data/b \
	--name count-zero \
	--bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
	--port 30334 \
	--ws-port 9945 \
	--rpc-port 9934 \
	--rpc-methods unsafe

subzero-c:
	./target/release/subzero-node \
	--validator \
	--chain local \
	--base-path ./data/c \
	--name mona-lisa-overdrive \
	--bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
	--port 30335 \
	--ws-port 9946 \
	--rpc-port 9935 \
	--rpc-methods unsafe

purge:
	rm -rf ./data/a
	rm -rf ./data/b
	rm -rf ./data/c
