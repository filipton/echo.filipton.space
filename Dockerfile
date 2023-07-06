FROM debian:bookworm-slim

COPY ./build /app
WORKDIR /app
RUN ulimit -n 50000
ENTRYPOINT ["/app/backend"]
