# Public Notes REST API (Rust)

A simple, robust public Notes REST API built with Rust, Actix-web, PostgreSQL, and Redis caching.

---

## Libraries & Ecosystem

Here are the key libraries (crates) chosen for this project and why:

### 1. Request Validation: `validator` (v0.19)
- **Crate**: [`validator`](https://crates.io/crates/validator) with `validator_derive`.
- **Purpose**: Declarative struct attribute-based validation.
- **Why it was chosen**: In the Rust ecosystem, `validator` is the industry standard for validating incoming JSON payloads. It enables compile-time generation of field checks (such as string length bounds `length(min = 1, max = 80)` for title and `length(min = 1, max = 512)` for content) with clean error formatting.

### 2. Database (PostgreSQL): `sqlx` (v0.8)
- **Crate**: [`sqlx`](https://crates.io/crates/sqlx) (features: `runtime-tokio-rustls`, `postgres`, `macros`, `chrono`, `uuid`).
- **Purpose**: Pure async SQL toolkit and connection pooling for PostgreSQL.
- **Why it was chosen**: Unlike heavy ORMs (such as Diesel), `sqlx` is completely asynchronous, uses non-blocking Tokio I/O, supports modern Rust types natively (`Uuid`, `chrono::DateTime`), and runs without requiring C OpenSSL system dependencies when using `rustls`.

### 3. Caching (Redis): `deadpool-redis` (v0.18) & `redis` (v0.27)
- **Crate**: [`deadpool-redis`](https://crates.io/crates/deadpool-redis) and [`redis`](https://crates.io/crates/redis) (with `tokio-comp`).
- **Purpose**: Asynchronous Redis client and connection pool.
- **Why it was chosen**:
  - `deadpool-redis` provides resilient connection pooling built on top of Tokio.
  - Enables sub-millisecond retrieval of cached note lists and individual notes.
  - Features transparent degradation: if Redis is offline or not configured, the service logs a warning and proceeds to serve queries directly from PostgreSQL without failing or crashing.

### 4. Web Framework: `actix-web` (v4.9)
- **Crate**: [`actix-web`](https://crates.io/crates/actix-web) and [`actix-cors`](https://crates.io/crates/actix-cors).
- **Purpose**: High-throughput, battle-tested HTTP server and routing.

---

## Note Data Model & Rules

| Field | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | UUID | Generated | Auto UUID v4 | Unique identifier |
| `title` | String | Yes | 1 to 80 characters | Trimmed automatically |
| `content` | String | Yes | 1 to 512 characters | Trimmed automatically |
| `color_front` | String | Optional | Max 50 chars, hex code | Text & icon color (e.g. `#1e293b`). Auto-assigned from random palette if omitted |
| `color_back` | String | Optional | Max 50 chars, hex code | Paper & background color (e.g. `#bfdbfe`). Auto-assigned from random palette if omitted |
| `created_at` | DateTime (ISO-8601) | Generated | UTC | Creation timestamp |
| `updated_at` | DateTime (ISO-8601) | Generated | UTC | Last modification timestamp |

---

## API Documentation

### 1. Health Check

#### `GET /api/healty`
Check status of API service, PostgreSQL, and Redis cache.

- **Response `200 OK`**:
```json
{
  "status": "healthy",
  "database": "connected",
  "redis": "connected"
}
```

---

### 2. List Notes (Cursor-based Pagination)

#### `GET /api/v1/lists`
Returns paginated note summaries with short content previews (`content_preview` up to 120 chars).
Uses high-performance **keyset cursor pagination** `(created_at, id) < (cursor_created_at, cursor_id)` backed by the B-Tree index `idx_notes_created_at_id`, ensuring $O(1)$ query complexity regardless of table size (even with millions of rows).

- **Query Parameters**:
  - `limit` (optional, integer, default: `50`, min: `1`, max: `100`): Maximum number of notes per page.
  - `next_id` (optional, UUID): Cursor ID obtained from `next_id` of the previous page to fetch the subsequent page.

- **Response `200 OK`**:
```json
{
  "data": [
    {
      "id": "257da34d-3fd7-40de-bb0a-4da708f80865",
      "title": "Meeting Notes",
      "content_preview": "Discuss Q4 architecture roadmap...",
      "color_front": "#1e293b",
      "color_back": "#bfdbfe",
      "created_at": "2026-11-10T05:32:53.258734Z",
      "updated_at": "2026-11-10T05:32:53.258734Z"
    }
  ],
  "is_next": true,
  "next_id": "1576e957-86cf-4a4c-9c16-318ab8826492"
}
```

- When on the final page, `"is_next"` will be `false` and `"next_id"` will be `null`:
```json
{
  "data": [...],
  "is_next": false,
  "next_id": null
}
```

---

### 3. Get Note Detail

#### `GET /api/v1/note/{id}`
Returns complete note detail by UUID.

- **Path Parameters**:
  - `id` (UUID): Note identifier.

- **Response `200 OK`**:
```json
{
  "data": {
    "id": "257da34d-3fd7-40de-bb0a-4da708f80865",
    "title": "Meeting Notes",
    "content": "Discuss Q4 architecture roadmap and containerization strategy.",
    "color_front": "#1e293b",
    "color_back": "#bfdbfe",
    "created_at": "2026-11-10T05:32:53.258734Z",
    "updated_at": "2026-11-10T05:32:53.258734Z"
  }
}
```

- **Response `404 Not Found`**:
```json
{
  "status": 404,
  "message": "Note with id 257da34d-3fd7-40de-bb0a-4da708f80865 not found"
}
```

---

### 4. Create Note

#### `POST /api/v1/note`
Creates a new note. All fields are trimmed before validation and storage. If `color_front` or `color_back` is omitted or empty, random hex colors will be assigned automatically.

- **Request Body**:
```json
{
  "title": "Weekly Standup",
  "content": "Sprint goals review and blocker removal.",
  "color_front": "#064e3b",
  "color_back": "#bbf7d0"
}
```

- **Response `201 Created`**:
```json
{
  "status": 201,
  "success": true,
  "message": "Note created successfully",
  "note_id": "257da34d-3fd7-40de-bb0a-4da708f80865",
  "data": {
    "id": "257da34d-3fd7-40de-bb0a-4da708f80865",
    "title": "Weekly Standup",
    "content": "Sprint goals review and blocker removal.",
    "color_front": "#064e3b",
    "color_back": "#bbf7d0",
    "created_at": "2026-11-10T05:32:53.258734Z",
    "updated_at": "2026-11-10T05:32:53.258734Z"
  }
}
```

- **Response `400 Bad Request`**:
```json
{
  "status": 400,
  "message": "Title must be between 1 and 80 characters"
}
```

---

### 5. Full Update (Replace Note)

#### `PUT /api/v1/note/{id}`
Replaces all note attributes with new values.

- **Request Body**:
```json
{
  "title": "Updated Standup Notes",
  "content": "All blockers have been resolved.",
  "color_front": "#1e1b4b",
  "color_back": "#e0e7ff"
}
```

- **Response `200 OK`**:
```json
{
  "status": 200,
  "success": true,
  "message": "Note updated successfully",
  "note_id": "257da34d-3fd7-40de-bb0a-4da708f80865",
  "data": {
    "id": "257da34d-3fd7-40de-bb0a-4da708f80865",
    "title": "Updated Standup Notes",
    "content": "All blockers have been resolved.",
    "color_front": "#1e1b4b",
    "color_back": "#e0e7ff",
    "created_at": "2026-11-10T05:32:53.258734Z",
    "updated_at": "2026-11-10T05:35:10.123456Z"
  }
}
```

---

### 6. Partial Update (Patch Note)

#### `PATCH /api/v1/note/{id}`
Partially updates specific note attributes (`title`, `content`, `color_front`, or `color_back`).

- **Request Body** (any subset of fields):
```json
{
  "title": "Patched Title Only",
  "color_front": "#ff00ff"
}
```

- **Response `200 OK`**:
```json
{
  "status": 200,
  "success": true,
  "message": "Note updated successfully",
  "note_id": "257da34d-3fd7-40de-bb0a-4da708f80865",
  "data": {
    "id": "257da34d-3fd7-40de-bb0a-4da708f80865",
    "title": "Patched Title Only",
    "content": "All blockers have been resolved.",
    "color_front": "#ff00ff",
    "color_back": "#e0e7ff",
    "created_at": "2026-11-10T05:32:53.258734Z",
    "updated_at": "2026-11-10T05:36:20.987654Z"
  }
}
```

---

### 7. Delete Note

#### `DELETE /api/v1/note/{id}`
Deletes a note by its UUID and invalidates associated Redis caches.

- **Response `200 OK`**:
```json
{
  "status": 200,
  "success": true,
  "message": "Note deleted successfully"
}
```

- **Response `404 Not Found`**:
```json
{
  "status": 404,
  "message": "Note with id 257da34d-3fd7-40de-bb0a-4da708f80865 not found"
}
```

---

### 8. Standard Error Format

Any route error, invalid JSON payload, or validation failure follows a unified error schema:

```json
{
  "status": 400,
  "message": "Title must be between 1 and 80 characters, Content must be between 1 and 512 characters"
}
```

---

## Running with Docker Compose

To start PostgreSQL, Redis, and the backend service:

```bash
cd app-api
docker compose up -d --build
```

The server will be reachable at `http://localhost:8080`.

To view service logs:
```bash
docker compose logs -f app-api
```

To stop containers:
```bash
docker compose down
```
