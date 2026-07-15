# Loona

Discord bot for Konserva

## Developing

1. Build the project

```bash
cargo build
```

1. Run in development mode

```bash
cargo run
```

## Building

1. Build an optimized release binary

```bash
cargo build --release
```

The compiled binary will be available at `target/release/loona`.

## Running with Docker

1. Build and start the container

```bash
docker compose up --build
```

On first run, `loona` will generate a default `config.toml` in the mounted `./data` directory — fill in the required fields (e.g. `discord.token`) before restarting.

1. Stop the container

```bash
docker compose down
```

## License

This project is licensed under the GNU General Public License v3.0. See the [LICENSE.md](./LICENSE.md) file for details.
