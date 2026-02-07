# 🚀 Tunex — Secure Tunnel Proxy in Rust

**Tunex** is a Rust project inspired by **ngrok/zrok**, enabling you to expose local services to the internet through a **secure, fast, and extensible tunnel**.

---

## 🧠 Overview

Tunex lets you run a **Client Agent** locally that **listens for connections** and registers its address with a public **Gateway Server**. The gateway stores tunnel metadata in **Redis** (shared across multiple gateway instances). When a user hits the gateway, it looks up the client address in Redis, opens **TCP** to the client, and forwards the request. The client forwards to the local service and returns the response.

Flow:

```
Internet → Tunex Gateway → Redis (lookup) → TCP → Tunex Client → Local Service
```

---

## 🎯 Project Goals

- Expose local services without opening ports on the router (client can be behind NAT if it has an advertised address)
- Support **multiple gateway instances** via Redis
- Forward HTTP over TCP (no WebSocket overhead)
- Prepare foundation for TLS, auth, keep-alive, and subdomains

---

## 🏗️ Tunex Architecture

Main components:

- **Tunex Gateway Server**
  - Ingress proxy (HTTP `/tunnel/:name/...`)
  - Redis-backed tunnel registry (tunnel name → client address)
  - POST `/register` for client registration
  - Auth (roadmap)
  - Metrics (roadmap)

- **Tunex Client Agent**
  - TCP listener (gateways connect here)
  - HTTP registration + heartbeat to gateway
  - Local proxy (forwards to localhost)

- **Redis**
  - Shared registry: `tunex:tunnel:{name}` → `host:port` with TTL
  - Enables any gateway instance to resolve and connect to the client

- **Local Service**
  - Application to be exposed (API, dashboard, site)

---

## 🧩 How the Tunnel Works

### 1️⃣ Client listens and registers

The client:

1. Binds TCP on `0.0.0.0:listen_port` (e.g. 9090).
2. Sends **POST /register** to the gateway with `tunnel_name` and `client_address` (e.g. `192.168.1.10:9090` or `hostname:9090`).
3. Gateway writes to Redis: `tunex:tunnel:{name}` → `client_address` with TTL (default 60s).
4. Client sends the same POST periodically (heartbeat) to refresh TTL.

### 2️⃣ User request

When someone accesses:

```
http://gateway:PORT/tunnel/<name>/...
```

Any gateway instance:

1. Receives the request.
2. **GET** from Redis: `tunex:tunnel:{name}` → client address.
3. Opens a **new TCP connection** to that address, sends the HTTP request, reads the response.
4. Returns the response to the user.

### 3️⃣ Protocol gateway–client

The protocol between gateway and client is **HTTP over TCP**: one full HTTP request, one full HTTP response, then the connection is closed. **Conexões persistentes (keep-alive) entre gateway e cliente são fase futura; o protocolo atual é uma conexão TCP por request.**

### 🔄 Data Flow

```
User → Gateway → Redis (get address) → TCP connect → Client → Local App
Local App → Client → TCP response → Gateway → User
```

---

## 🧪 Current Tunex MVP

The MVP delivers:

- **TCP tunnel** (gateway connects to client; no WebSocket).
- **Redis** as shared registry for N gateways.
- **POST /register** with optional `token` / `Authorization` (ignored in MVP). **Registro é não autenticado no MVP; autenticação (token, mTLS) está prevista para fase posterior e o desenho da API não a bloqueia.**
- Basic Gateway and Client.
- One TCP connection per request (keep-alive is a future phase).

---

## ▶️ How to Run Tunex

#### 1️⃣ Start Redis (and optionally Postgres)

```bash
docker-compose up -d
```

#### 2️⃣ Start a local service

```bash
python3 -m http.server 3000
```

#### 3️⃣ Start the gateway

```bash
cd gateway
cargo run
```

Uses `REDIS_URL` (default `redis://127.0.0.1:6379`), `PORT` (default 8080), `TUNEX_TUNNEL_TTL_SECS` (default 60).

#### 4️⃣ Start the client

The client must be reachable at the address it advertises (same machine or network where gateways can connect).

```bash
cd client
cargo run -- --port 3000 --listen 9090 --advertise-address 127.0.0.1:9090 --gateway http://localhost:8080 --name default
```

- `--port`: local service port (3000).
- `--listen`: port the client listens on for gateway connections (9090).
- `--advertise-address`: address gateways will use to connect (e.g. `127.0.0.1:9090` or `hostname:9090`).
- `--gateway`: gateway base URL for POST /register.
- `--name`: tunnel name (path will be `/tunnel/<name>/...`).
- `--token`: optional; ignored by gateway in MVP.

#### 5️⃣ Test

```bash
curl http://localhost:8080/tunnel/default/
```

---

## 🧱 Tunex Protocol (Roadmap)

Future frame structure for multiplexing:

```
[stream_id][type][length][payload]
```

---

## 🔐 Security (Roadmap)

- **MVP:** Registration is unauthenticated; optional `token` / `Authorization` accepted and ignored.
- TLS between client and gateway
- mTLS
- JWT / API key validation on POST /register
- ACL per tunnel
- Rate limiting

---

## 📈 Scalability

- **Registry in Redis** — multiple gateway instances share the same Redis; any instance can resolve and connect to the client.
- Clustered gateway behind a load balancer (no sticky session required).
- Optional: sharding by tunnel, connection pooling / keep-alive (future).

---

## 🗺️ Tunex Roadmap

### Phase 1 — MVP

- [x] TCP tunnel (gateway connects to client)
- [x] Redis registry
- [x] Basic Gateway (POST /register + proxy)
- [x] Basic Client (listen + register + heartbeat)

### Phase 2 — Real Proxy

- HTTP proxy (full)
- Multiplexing
- **Keep-alive** (persistent connections gateway–client)

### Phase 3 — Product

- TLS
- Subdomains
- **Auth** (token / mTLS on registration)
- Dashboard
- Metrics

---

## ✨ Final Usage (Vision)

```bash
tunex expose 3000
```

Output:

```
Public URL: https://abc123.tunex.dev
```
