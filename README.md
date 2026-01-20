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

## Quick Start

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

# Required - Generate a secure token
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

## Environment Configuration

This project uses a **centralized environment configuration**. A single `.env` file in the root directory configures both services.

### File Structure

```
BlueprintX-backend/
├── .env.example          # Main config - use for Docker deployments
├── .env                   # Your local config (git-ignored)
├── rust/.env.example     # For local dev only (cargo run)
└── python/.env.example   # For local dev only (uv run)
```

### For Docker (Recommended)

Just use the root `.env` file. Docker Compose automatically:
- Constructs database URLs with correct prefixes for each service
- Routes shared variables (like `INTERNAL_API_TOKEN`) to both services
- Sets up Redis with separate databases (db 0 for Rust, db 1 for Python)

### For Local Development

When running services outside Docker (e.g., `cargo run` or `uvicorn`):

1. Copy the service-specific `.env.example` to `.env` in that directory
2. Update hostnames from Docker service names to `localhost`

### Required Variables

| Variable | Description | Where to get |
|----------|-------------|--------------|
| `GEMINI_API_KEY` | Google Gemini API key | [Google AI Studio](https://makersuite.google.com/app/apikey) |
| `SUPABASE_JWT_JWKS_URL` | Supabase JWKS URL | Supabase Dashboard > Settings > API |
| `SUPABASE_JWT_ISSUER` | Supabase JWT issuer | Supabase Dashboard > Settings > API |
| `INTERNAL_API_TOKEN` | Shared secret between services | Generate: `openssl rand -base64 32` |

### Optional Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ENV` | `dev` | Environment (dev/staging/prod) |
| `POSTGRES_*` | `postgres` | Database credentials and host |
| `REDIS_HOST` | `redis` | Redis hostname |
| `RUST_LOG` | `info` | Rust log level |
| `LOG_LEVEL` | `INFO` | Python log level |
| `CORS_ALLOW_ORIGINS` | `http://localhost:3000` | Allowed CORS origins |
| `GEMINI_MODEL_*` | `gemini-2.5-flash` | Gemini model overrides |
| `CHUNK_SIZE` | `1000` | Document chunk size for embeddings |
| `MAX_UPLOAD_SIZE_MB` | `100` | Max file upload size |

See `.env.example` for the complete list with descriptions.

## Docker Compose Commands

### Starting Services

```bash
# Build and start all services (first time or after changes)
docker compose up --build

# Start in detached mode (background)
docker compose up -d

# Start without rebuilding (faster, use after initial build)
docker compose up

# Start specific services only
docker compose up db redis              # Just infrastructure
docker compose up db redis ai-service   # Without Rust API
```

### Stopping Services

```bash
# Stop all services (keeps data)
docker compose down

# Stop and remove volumes (WARNING: deletes all data)
docker compose down -v

# Stop specific service
docker compose stop rust-api
```

### Viewing Logs

```bash
# View all logs
docker compose logs

# Follow logs in real-time
docker compose logs -f

# View specific service logs
docker compose logs -f rust-api
docker compose logs -f ai-service

# View last 100 lines
docker compose logs --tail 100 rust-api
```

### Rebuilding Services

```bash
# Rebuild specific service
docker compose build rust-api
docker compose build ai-service

# Rebuild and restart
docker compose up --build rust-api

# Force rebuild without cache
docker compose build --no-cache rust-api
```

### Database Operations

```bash
# Access PostgreSQL shell
docker compose exec db psql -U postgres -d blueprintx

# Run SQL file
docker compose exec -T db psql -U postgres -d blueprintx < script.sql

# Backup database
docker compose exec db pg_dump -U postgres blueprintx > backup.sql

# Restore database
docker compose exec -T db psql -U postgres -d blueprintx < backup.sql
```

### Redis Operations

```bash
# Access Redis CLI
docker compose exec redis redis-cli

# Clear all Redis data
docker compose exec redis redis-cli FLUSHALL
```

### Useful Commands

```bash
# Check service status
docker compose ps

# View resource usage
docker compose top

# Restart a service
docker compose restart rust-api

# Execute command in running container
docker compose exec rust-api /bin/sh
docker compose exec ai-service /bin/bash
```

## Development

### Rust API

```bash
cd rust
cp .env.example .env  # Update with localhost URLs

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
cp .env.example .env  # Update with localhost URLs

# Using uv (recommended)
uv sync
uv run uvicorn app.main:app --reload --port 8000

# Or using pip
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
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
├── docker-compose.yml       # Service orchestration
├── init-db.sql              # pgvector setup
├── .env.example             # Centralized env config
│
├── rust/                    # Rust API
│   ├── Cargo.toml
│   ├── Dockerfile
│   ├── .env.example         # Local dev config
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
    ├── .env.example         # Local dev config
    └── app/
        ├── main.py          # FastAPI app
        ├── config.py        # Pydantic Settings
        ├── gemini/          # Gemini client, embeddings
        ├── graphs/          # LangGraph pipelines
        ├── vectorstore/     # pgvector implementation
        ├── documents/       # PDF processing
        ├── jobs/            # Async job system
        └── routes/          # API endpoints
```

## Branches

- `main` - Production
- `staging` - Testing/QA
