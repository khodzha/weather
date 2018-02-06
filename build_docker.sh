docker run --env-file .dockerenv --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder cargo build --release
docker build -t weather .
