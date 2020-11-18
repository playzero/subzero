FROM debian:stretch-slim

# metadata
ARG VCS_REF
ARG BUILD_DATE

LABEL io.zero.image.authors="devops@zero.io" \
	io.zero.image.vendor="ZERO Technologies" \
	io.zero.image.title="playzero/subzero" \
	io.zero.image.description="subzero. for videogames." \
	io.zero.image.source="https://github.com/playzero/subzero/blob/${VCS_REF}/.builder/docker/Dockerfile" \
	io.zero.image.revision="${VCS_REF}" \
	io.zero.image.created="${BUILD_DATE}" \
	io.zero.image.documentation="https://wiki.zero.io/subzero"

# show backtraces
ENV RUST_BACKTRACE 1

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get upgrade -y && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
		libssl1.1 \
		ca-certificates \
		curl && \
# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
# add user
	useradd -m -u 1000 -U -s /bin/sh -d /subzero subzero

# add subzero binary to docker image
COPY ./subzero /usr/local/bin

USER subzero

# check if executable works in this container
RUN /usr/local/bin/subzero --version

EXPOSE 30333 9933 9944
VOLUME ["/subzero"]

ENTRYPOINT ["/usr/local/bin/subzero"]

