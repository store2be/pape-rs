FROM debian:jessie-slim

WORKDIR /papers

# poppler-utils: pdfunite
# imagemagick: convert
RUN apt-get update -y && apt-get install -y \
    wget \
    libpod-pom-perl \
    fontconfig \
    fonts-lmodern \
    poppler-utils \
    imagemagick \
    && rm -rf /var/lib/apt

# Install a small version of TeXLive with the profile from texlive.profile
RUN echo '4377c56b55425691c73f8ca96e1814c3f412ff909d23d2ce3cf0c755420d3e9d6d761c398ea7696519e2a827ccb5384c00fc63e9860f916d0ddb090ad39e900e  install-tl-unx.tar.gz\n' > texlive.sha512
RUN wget 'http://mirror.ctan.org/systems/texlive/tlnet/install-tl-unx.tar.gz'
RUN sha512sum -c texlive.sha512
RUN tar xzf install-tl-unx.tar.gz
COPY docker/texlive.profile texlive.profile
RUN ./install-tl-20180503/install-tl -profile=texlive.profile

RUN apt-get update -y && \
    apt-get install -y curl libssl-dev libssl1.0.0 openssl && \
    rm -rf /var/lib/apt/lists/

# Required because openssl can't be located automatically
ENV OPENSSL_INCLUDE_DIR=/usr/include
ENV OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu/
