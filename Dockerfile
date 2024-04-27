FROM rust:latest as builder

WORKDIR /usr/src/bimhd

COPY . .

RUN cargo build --release

FROM debian:latest

RUN apt-get update && apt-get install -y \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

WORKDIR /usr/src/bimhd

COPY --from=builder /usr/src/bimhd/target/release/bimhd .
COPY --from=builder /usr/src/bimhd/gtfs.zip .

EXPOSE 3000

CMD ["./bimhd"]