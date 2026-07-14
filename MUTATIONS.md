# External mutations log

## In-repo additions sourced from outside (provenance)

These are writes **into** this repo (allowed), recorded so the fixtures are
regenerable and their origin is traceable.

- `crates/anchor-litesvm/tests/fixtures/mpl_core.so` (810,504 bytes, sha256
  `4a5cc50d…`) copied from `~/oss/mine/vogon-registry/mpl-core/fixtures/mpl_core.so`,
  itself a mainnet dump of program `CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d`
  (`solana program dump -u m`). Needed as the mpl-core CPI callee for the stake
  example.
