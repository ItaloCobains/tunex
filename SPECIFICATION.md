# 🚀 Tunex — Secure Tunnel Proxy in Rust

**Tunex** is a Rust project inspired by **ngrok/zrok**, enabling you to expose local services to the internet through a **secure, fast, and extensible tunnel**.

---

## 🧠 Overview

Tunex lets you run a **Client Agent** locally and connect it to a public **Gateway Server**.  
The server creates a public endpoint and forwards all incoming traffic to the user's local service via the tunnel.

Flow:

```
Internet → Tunex Gateway → Tunnel → Tunex Client → Local Service
```

---

## 🎯 Project Goals

- Expose local services without opening ports on the router  
- Create persistent tunnels  
- Forward HTTP/TCP traffic  
- Implement multiplexing  
- Prepare foundation for TLS, auth, and subdomains  

---

## 🏗️ Tunex Architecture

Main components:

- **Tunex Gateway Server**
  - Ingress Proxy
  - Tunnel Registry
  - Auth (roadmap)
  - Metrics (roadmap)

- **Tunex Client Agent**
  - Connection Manager
  - Protocol Handler
  - Local Proxy

- **Local Service**
  - Application to be exposed (API, dashboard, site)

---

## 🧩 Project Structure

### 🔌 How the Tunnel Works

#### 1️⃣ Connection

The client connects to the gateway:

```
ws://gateway:PORT/ws
```

Tunex keeps the connection open and registers the tunnel.

#### 2️⃣ Registration

The client sends:

- Local port  
- Service type (HTTP/TCP)  
- Tunnel name (future: subdomain)  

The gateway creates a public endpoint.

#### 3️⃣ Forwarding

When someone accesses:

```
https://abc.tunex.dev
```

The gateway:

1. Receives the request  
2. Finds the tunnel  
3. Sends it over WebSocket  
4. Client forwards to localhost  
5. Response travels back through the tunnel  

### 🔄 Data Flow

```
User → Tunex Gateway → Tunnel → Tunex Client → Local App
Local App → Tunex Client → Tunnel → Tunex Gateway → User
```

---

## 🧪 Current Tunex MVP

The MVP delivers:

- WebSocket tunnel  
- Basic Tunex Gateway  
- Tunex Client agent  
- Simple forwarding  
- Base for multiplexing  

---

## ▶️ How to Run Tunex

#### 1️⃣ Start a local service

```bash
python3 -m http.server 3000
```

#### 2️⃣ Start the gateway

```bash
cd gateway
cargo run
```

#### 3️⃣ Start the client

```bash
cd client
cargo run
```

#### 4️⃣ Test

```bash
curl http://localhost:8080
```

---

## 🧱 Tunex Protocol (Roadmap)

Future frame structure:

```
[stream_id][type][length][payload]
```

---

## 🔐 Security (Roadmap)

- TLS between client and gateway  
- mTLS  
- JWT tokens  
- ACL per tunnel  
- Rate limiting  

---

## 📈 Scalability

- Registry in Redis  
- Clustered gateway  
- Load balancer  
- Sharding by tunnel  

---

## 🗺️ Tunex Roadmap

### Phase 1 — MVP

- [x] WebSocket tunnel  
- [x] Basic Gateway  
- [x] Basic Client  

### Phase 2 — Real Proxy

- HTTP proxy (full)  
- Multiplexing  
- Keep-alive  

### Phase 3 — Product

- TLS  
- Subdomains  
- Auth  
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
