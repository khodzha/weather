FROM scratch

ADD target/x86_64-unknown-linux-musl/release/weather /
EXPOSE 1337

CMD ["/weather"]
