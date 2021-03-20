# DataVault
A modern, centralized data managing and sharing solution

## Compatible cliens
- [CLI](https://github.com/DataManager-Go/DataManagerCLI)

# Requirements
- Some spare storage
- A Postgres database

# Setup

### Required
- Cargo
- Diesel (with postgres feature)

### Setup
```bash
echo DATABASE_URL=postgres://username:password@localhost/datavault > .env
diesel setup
diesel migration run
cargo build --release
```
After a successful build, the binary will be located at `./target/release/dv_server`<br>

### Configuration
Have a look at the example config: https://github.com/DataManager-Go/DataVault/blob/master/config.example.toml
