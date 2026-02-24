# xd-jupiter (Jupiter AMM adapter crate)

This crate implements:

- `LaunchpadAmm` for bonding curves (`XDBC3FsUpjDnYCcPBEgniLo4M13Wsiu8yLbcwcz2zqV`)
- `XDSwapAmm` for pools (`XDSwtQ2qNdjT4HsToizoAUwz3wAWL5nAhChQTEcv1Uh`)

- Crate compiles and all unit tests pass.
- Program IDs registered in Jupiter AMM loader map.
- `jupiter-core` compiles with this integration.
- Fixtures (program dumps + account snapshots) included.
- `test_amms.rs` simulation entries use shared PumpSwap variants.
