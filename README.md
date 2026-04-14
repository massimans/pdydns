# pdydns

`pdydns` watches your public IP and keeps PowerDNS `A` records updated.

## Configuration

Copy `config.example.toml` to `config.toml` and adjust the values.

Environment variables override the config file.

If you set all required env vars, you can omit the config file entirely.

Supported env vars:

- `PDYDNS_POWERDNS_API_URL`
- `PDYDNS_POWERDNS_API_KEY`
- `PDYDNS_POWERDNS_SERVER_ID`
- `PDYDNS_INTERVAL_SECONDS`
- `PDYDNS_DEFAULT_TTL`
- `PDYDNS_PUBLIC_IP_URL`
- `PDYDNS_RECORDS` as a JSON array of `{ zone, name, ttl? }`

## Run locally

```bash
PDYDNS_POWERDNS_API_KEY=change-me cargo run -- --config config.toml
```

## Container

The repository publishes a container image to GitHub Container Registry on pushes to `main` and version tags.

```bash
docker run --rm -e PDYDNS_POWERDNS_API_KEY=change-me -v "$PWD/config.toml:/config.toml:ro" ghcr.io/massimans/pdydns:latest --config /config.toml
```
