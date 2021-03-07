# DataVault
A modern, centralized data managing and sharing solution

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

