FROM debian:jessie-slim

RUN apt-get update -y && \
    apt-get install --no-install-recommends -y texlive-xetex && \
    rm -rf /var/lib/apt/lists/

RUN apt-get update -y && \
    apt-get install curl && \
    rm -rf /var/lib/apt/lists/

WORKDIR /papers

COPY target/release .

CMD ./papers-server
