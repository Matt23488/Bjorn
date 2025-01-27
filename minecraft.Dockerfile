FROM ubuntu:24.04

RUN apt update && apt install -y openjdk-21-jre && apt install -y libc6

WORKDIR /app

COPY ../target/release/game_manager .
CMD ./game_manager
