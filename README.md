A production-grade Solana blockchain indexer built in Rust. Indexes real-time transactions from the Raydium AMM program using WebSocket subscriptions, stores them in PostgreSQL, and exposes a REST API.


- **Listener** — persistent WebSocket connection to Solana RPC with auto-reconnect and exponential backoff
- **Parser** — fetches full transaction details via HTTP RPC, decodes instruction types from logs
- **Database** — PostgreSQL with 5 tables: transactions, instructions, accounts, events, slot_checkpoints
- **API** — Axum REST API with 5 endpoints

## Tech Stack

- Rust + Tokio (async runtime)
- tokio-tungstenite (WebSocket client)
- reqwest (HTTP RPC calls)
- SQLx + PostgreSQL (database)
- Axum (REST API)

## Setup

### Prerequisites
- Rust 1.70+
- PostgreSQL 14+

### Install

```bash
git clone https://github.com/YOUR_USERNAME/solana-indexer
cd solana-indexer
```

### Configure

Create a `.env` file:
```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/solana_indexer
RUST_LOG=solana_indexer=info
```

### Database

```bash
psql -U postgres -h localhost -c "CREATE DATABASE solana_indexer;"
psql -U postgres -h localhost -d solana_indexer -f migrations/001_initial.sql
```

### Run

```bash
cargo run
```

## API Endpoints

| Endpoint | Description |
|---|---|
| `GET /health` | Indexer status and last indexed slot |
| `GET /stats` | Transaction, account, and event counts |
| `GET /transactions?limit=20&offset=0` | List recent transactions |
| `GET /transactions?account=<pubkey>` | Transactions for a specific account |
| `GET /accounts/:pubkey/history` | Account activity history |
| `GET /events?type=swap&limit=20` | Decoded program events |

## Example Responses

### GET /health
```json
{
  "status": "ok",
  "program_id": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
  "last_indexed_slot": 426235464,
  "uptime_seconds": 85
}
```

### GET /stats
```json
{
  "transactions": 150,
  "accounts": 693,
  "events": 23,
  "success_rate": 0.717
}
```

## Program

Currently indexing **Raydium AMM** (`675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`) — one of Solana's largest DEXes by volume. Change `program_id` in `config/default.toml` to index any Solana program.
EOF
