FROM debian:bookworm-slim

RUN echo "deb http://security.debian.org/debian-security bullseye-security main" > /etc/apt/sources.list
RUN apt update && apt install -y libssl1.1 ca-certificates
RUN apt clean

COPY ./build /app
WORKDIR /app
RUN ulimit -n 50000
ENTRYPOINT ["/app/backend"]
