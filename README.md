# Matrix Bot

My Matrix Bot. It can join, leave and remind.

## Usage

Copy `config.sample.yaml` to `config.yaml` and run with `cargo run`. You can also use `APP__LOGIN__PASSWORD` for providing the password (and similar the respective environment variable for the other config options).

Alternatively, there is the possibility to build the docker image and use it for running:

```bash
docker build . -t yourtagname
docker run --rm -v /path/to/config:/opt/app/config.yaml yourtagname
# or use mine:
docker run --rm -v /path/to/config:/opt/app/config.yaml flixcoder/matrix-bot
```

## Lints

This projects uses a bunch of clippy lints for higher code quality and style.

Install [`cargo-lints`](https://github.com/soramitsu/iroha2-cargo_lints) using `cargo install --git https://github.com/FlixCoder/cargo-lints`. The lints are defined in `lints.toml` and can be checked by running `cargo lints clippy --all-targets --workspace`.
