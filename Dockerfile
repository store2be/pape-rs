FROM debian:jessie-slim

RUN apt-get update -y && \
    apt-get install --no-install-recommends -y texlive-xetex fonts-lmodern && \
    rm -rf /var/lib/apt/lists/

RUN apt-get update -y && \
    apt-get install -y curl libssl-dev libssl1.0.0 openssl && \
    rm -rf /var/lib/apt/lists/

# Required because openssl can't be located automatically
ENV OPENSSL_INCLUDE_DIR=/usr/include
ENV OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu/

WORKDIR /papers

COPY target/release/papers-server .

CMD ./papers-server
