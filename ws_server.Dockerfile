FROM ubuntu:22.04

WORKDIR /app

COPY target/release/ws_server .

CMD ./ws_server
