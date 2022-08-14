FROM ubuntu:22.10
WORKDIR /app
ARG UID=1000
ARG GID=1000
RUN apt-get update && apt-get install -y gcc g++ cmake git libboost-dev libboost-serialization-dev libz-dev curl
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
