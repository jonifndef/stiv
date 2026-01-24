from rust:latest

ARG DEBIAN_FRONTEND=noninteractive

RUN apt update && apt install -y \
    libchafa-dev
