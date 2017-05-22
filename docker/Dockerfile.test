FROM store2be/pape-rs-base

RUN apt-get update -y && apt-get install -y build-essential

WORKDIR /papers

ENV CARGO_HOME /papers/.cargo
ENV PATH=/papers/.cargo/bin:$PATH
ENV CHANNEL=nightly

RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain $CHANNEL -y
RUN rustup default $CHANNEL

ENV PATH=/root/.rustup/toolchains/${CHANNEL}-x86_64-unknown-linux-gnu/bin:$PATH

CMD cargo test
