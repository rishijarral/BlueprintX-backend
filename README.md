# BlueprintX Backend

Backend services for BlueprintX - a construction project management platform for General Contractors.

## Architecture

```
┌─────────────────┐     ┌─────────────────┐
│   Frontend      │────▶│   Rust API      │──────┐
│   (Next.js)     │     │   (Axum)        │      │
└─────────────────┘     │   :8080         │      ▼
                        └────────┬────────┘  ┌───────┐
                                 │           │ Redis │
                                 ▼           └───────┘
                        ┌─────────────────┐      │
                        │  Python AI      │──────┘
                        │  (FastAPI)      │
                        │  :8000          │
                        └────────┬────────┘
                                 │
                                 ▼
                        ┌─────────────────┐
                        │  PostgreSQL     │
                        │  (pgvector)     │
                        └─────────────────┘
```

- **Rust API** - Main backend. Handles auth, projects, tenders, bids, documents. Proxies AI requests.
- **Python AI Service** - Internal service for Gemini LLM. Plan analysis, trade scopes, RAG Q&A.
- **PostgreSQL + pgvector** - Database with vector embeddings for document search.
- **Redis** - Caching for both services.

## Prerequisites

- Docker & Docker Compose
- Supabase project (for auth)
- Google Gemini API key

## Setup

### 1. Clone and configure

```bash
git clone <repo-url>
cd BlueprintX-backend
cp .env.example .env
```

### 2. Edit `.env`

```bash
# Required - Get from Supabase dashboard
SUPABASE_JWT_JWKS_URL=https://YOUR_PROJECT.supabase.co/auth/v1/.well-known/jwks.json
SUPABASE_JWT_ISSUER=https://YOUR_PROJECT.supabase.co/auth/v1

# Required - Get from Google AI Studio
GEMINI_API_KEY=your-gemini-api-key

# Required - Generate a random token (must match between services)
INTERNAL_API_TOKEN=$(openssl rand -base64 32)
```

### 3. Start services

```bash
docker compose up --build
```

Services will be available at:
- Rust API: http://localhost:8080
- Python AI: http://localhost:8000 (internal only)
- PostgreSQL: localhost:5432
- Redis: localhost:6379

### 4. Verify

```bash
# Check Rust API health
curl http://localhost:8080/api/health

# Expected response:
# {"status":"healthy","version":"0.1.0","services":{"database":"ok","redis":"ok","ai_service":"ok"}}
```

## Development

### Rust API

```bash
cd rust

# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Run locally (needs DB and Redis running)
cargo run

# Check for issues
cargo check

# Run tests
cargo test
```

### Python AI Service

```bash
cd python

# Create virtualenv
python -m venv .venv
source .venv/bin/activate

# Install dependencies
pip install -r requirements.txt
pip install -r requirements-dev.txt  # for linting/testing

# Run locally
uvicorn app.main:app --reload --port 8000

# Lint
ruff check app/

# Type check
pyright app/
```

## API Endpoints

### Public (Rust API)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/health` | Health check |
| GET | `/api/me` | Current user info |
| GET | `/api/projects` | List projects |
| POST | `/api/projects` | Create project |
| GET | `/api/projects/:id` | Get project |
| GET | `/api/projects/:id/documents` | List documents |
| POST | `/api/projects/:id/ai/summary` | Generate plan summary |
| POST | `/api/projects/:id/ai/trade-scopes` | Extract trade scopes |
| POST | `/api/projects/:id/ai/qna` | Ask question about docs |

All endpoints except `/api/health` require `Authorization: Bearer <supabase-jwt>` header.

### Internal (Python AI Service)

Not exposed externally. Called by Rust API with `X-Internal-Token` header.

## Project Structure

```
.
├── docker-compose.yml
├── init-db.sql              # pgvector setup
├── .env.example
│
├── rust/                    # Rust API
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/
│       ├── main.rs
│       ├── app.rs           # AppState, router
│       ├── config.rs        # Environment config
│       ├── auth/            # Supabase JWT auth
│       ├── routes/          # API endpoints
│       ├── services/        # Redis cache, AI client
│       └── domain/          # Data models
│
└── python/                  # Python AI Service
    ├── requirements.txt
    ├── Dockerfile
    └── app/
        ├── main.py          # FastAPI app
        ├── config.py        # Settings
        ├── gemini/          # Gemini client, embeddings
        ├── graphs/          # LangGraph pipelines
        ├── vectorstore/     # pgvector implementation
        ├── documents/       # PDF processing
        ├── jobs/            # Async job system
        └── routes/          # API endpoints
```

## Environment Variables

### Required

| Variable | Description |
|----------|-------------|
| `GEMINI_API_KEY` | Google Gemini API key |
| `SUPABASE_JWT_JWKS_URL` | Supabase JWKS URL for JWT verification |
| `SUPABASE_JWT_ISSUER` | Supabase JWT issuer URL |
| `INTERNAL_API_TOKEN` | Shared secret between Rust and Python |

### Optional

| Variable | Default | Description |
|----------|---------|-------------|
| `ENV` | `dev` | Environment (dev/staging/prod) |
| `RUST_LOG` | `info` | Rust log level |
| `LOG_LEVEL` | `INFO` | Python log level |
| `CORS_ALLOW_ORIGINS` | `http://localhost:3000` | Allowed CORS origins |

## Branches

- `main` - Production
- `staging` - Testing/QA
