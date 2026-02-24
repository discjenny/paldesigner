# Server Development

## Prerequisites
- Rust toolchain installed.
- PostgreSQL running on `localhost:5432`.
- Database user/password: `postgres:postgres`.

## Database Setup
```powershell
$env:PGPASSWORD='postgres'
psql -h localhost -U postgres -d postgres -c "CREATE DATABASE paldesigner;"
```

If the database already exists, PostgreSQL will return an error; that is expected and safe.

## Environment
Copy `.env.example` to `.env` and keep:

```env
APP_HOST=127.0.0.1
APP_PORT=8080
DATABASE_URL=postgres://postgres:postgres@localhost:5432/paldesigner
RUST_LOG=info
```

## Run
```powershell
cargo run
```

## Quick Checks
```powershell
curl http://127.0.0.1:8080/health
curl http://127.0.0.1:8080/ready
```
