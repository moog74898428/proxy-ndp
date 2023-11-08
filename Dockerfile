FROM debian
RUN apt update && \
    apt install -y git curl build-essential && \
    mkdir /work && \
    curl -sSf https://sh.rustup.rs | sh -s -- -y
ADD . /work
WORKDIR /work
RUN /root/.cargo/bin/cargo build --release

FROM gcr.io/distroless/cc-debian12:latest
COPY --from=0 /work/target/release/proxy-ndp /
ENTRYPOINT [ "/proxy-ndp" ]