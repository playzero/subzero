ARG BASE_IMAGE=phusion/baseimage:0.11
FROM $BASE_IMAGE

LABEL maintainer="devops@zero.io"

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
	apt-get dist-upgrade -y -o Dpkg::Options::="--force-confold" && \
	apt-get install -y cmake pkg-config libssl-dev git clang

RUN	curl https://sh.rustup.rs -sSf | sh -s -- -y && \
	export PATH="$PATH:$HOME/.cargo/bin" && \
	rustup toolchain install nightly-2020-10-01 && \
	rustup default nightly-2020-10-01 &&\
	rustup target add wasm32-unknown-unknown --toolchain nightly-2020-10-01 && \
	rustup show

CMD ["/bin/bash"]
