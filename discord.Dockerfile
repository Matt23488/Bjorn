FROM ubuntu:22.04

WORKDIR /app

COPY target/release/discord .

CMD ./discord
