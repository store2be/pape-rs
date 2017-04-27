#!/bin/sh

export DATETIME=$(date -u +%Y-%m-%dT%H-%M-%S)

docker build -t store2be/pape-rs-test -f Dockerfile.test . && \
    docker run --rm -it -v `pwd`:/papers store2be/pape-rs-test cargo clean && \
    docker run --rm -it -v `pwd`:/papers store2be/pape-rs-test cargo build --release --bin papers-server && \
    cp target/release/papers-server . && \
    docker build -t store2be/pape-rs:$DATETIME . && \
    rm papers-server && \
    echo $DATETIME > latest
