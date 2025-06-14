**disclaimer: ❗** this tunneling solution is intended for development and testing purposes. while functional, it may not include all security features required for production environments. use at your own discretion and implement additional security measures as needed for production deployments.

# funnel 🕳️

![go](https://img.shields.io/badge/Go-%2300ADD8.svg?style=for-the-badge&logo=go&logoColor=white)
![version](https://img.shields.io/badge/version-0.0.1a-blue.svg?style=for-the-badge)

a lightweight, high-performance tunneling solution implemented in go, designed to expose local services to the internet through secure websocket connections. this tool allows developers to share local development servers, test webhooks, and demonstrate applications without complex network configuration.

## about 📖

a lightweight, high-performance tunneling solution that creates secure connections between local services and remote clients through a central server. built with go and websockets, it provides fast, reliable tunneling with automatic reconnection and comprehensive logging. perfect for development, testing, and demonstration purposes.

## getting started 🚀

### prerequisites

- go 1.21 or later
- make (for build automation)

### installation

1. clone the repository:
   ```bash
   git clone https://github.com/karol-broda/funnel.git
   cd funnel
   ```

2. set up the development environment:
   ```bash
   make dev-setup
   ```

3. build both client and server:
   ```bash
   make build
   ```

## usage 💻

### checking version information 📋

both binaries support version information:

```bash
./bin/funnel version
./bin/funnel-server version

# or using make
make version
```

### running the funnel server 🖥️

start the funnel server to accept incoming tunnel connections:

```bash
./bin/funnel-server
```

by default, the server listens on port `8080`. you can specify a different port:

```bash
./bin/funnel-server -port 9000
```

### running the funnel client 📱

connect your local service to the funnel server using the new syntax:

```bash
# using just a port (will connect to localhost:3000)
./bin/funnel http 3000 --server http://localhost:8080

# using full address:port
./bin/funnel http localhost:3000 --server http://localhost:8080

# using a different address
./bin/funnel http 0.0.0.0:8080 --server http://localhost:8080
```

the client will automatically generate a tunnel id. you can also specify a custom tunnel id:

```bash
./bin/funnel http 3000 --server http://localhost:8080 --id my-custom-tunnel
```

## example usage 🎯

1. **start your local service:**
   ```bash
   python3 -m http.server 3000
   ```

2. **start the funnel server:**
   ```bash
   make run-server
   # or directly: ./bin/funnel-server -port 8080
   ```

3. **connect the funnel client:**
   ```bash
   ./bin/funnel http 3000 --server http://localhost:8080 --id demo
   ```

4. **access your service:**
   ```bash
   curl http://demo.localhost:8080
   ```

## development 🔧

### dependency management

the project uses go workspaces with automatic module discovery. here are the key dependency management commands:

#### `make tidy` vs `make deps-install`

**`make tidy`** - dependency cleanup and fixes:
- 🧹 cleans up `go.mod` files (adds missing, removes unused dependencies)
- 📝 updates dependency declarations across all modules
- 🔄 synchronizes the go workspace
- ✅ fixes ide linting errors like "package not in go.mod file"

```bash
make tidy  # fast, just updates go.mod files
```

**`make deps-install`** - complete dependency setup:
- 🧹 everything `make tidy` does (runs tidy first)
- ⬇️ downloads all dependencies to local cache
- 📦 prepares for offline development
- 🚀 complete setup for fresh installations

```bash
make deps-install  # slower, downloads everything
```

#### when to use which command:

```bash
# you added a new import to your code
make tidy  # ← fixes go.mod files, resolves ide errors

# fresh project setup or ci/cd
git clone https://github.com/karol-broda/funnel
cd funnel
make deps-install  # ← complete setup, ready to build

# regular maintenance
make tidy  # ← fast dependency cleanup

# preparing for offline work
make deps-install  # ← downloads all dependencies locally
```

#### other useful commands:

```bash
make list-modules   # show all discovered modules
make dev-setup      # complete development environment setup
make help          # show all available commands
```

### building for different platforms

create release binaries for multiple platforms:

```bash
make release
```

this builds binaries for:
- linux/amd64, linux/arm64
- darwin/amd64, darwin/arm64
- windows/amd64

binaries are created in the `dist/` directory.

## architecture 🏗️

the funnel system consists of three main components:

```
┌─────────────────┐    websocket     ┌─────────────────┐    http/https    ┌─────────────────┐
│                 │ ◄──────────────► │                 │ ◄──────────────► │                 │
│  funnel client  │                  │  funnel server  │                  │  remote client  │
│                 │                  │                 │                  │                 │
└─────────────────┘                  └─────────────────┘                  └─────────────────┘
         │                                     │                                     │
         ▼                                     ▼                                     ▼
┌─────────────────┐                  ┌─────────────────┐                  ┌─────────────────┐
│  local service  │                  │ tunnel registry │                  │   public url    │
│ localhost:3000  │                  │   & routing     │                  │ id.server:port  │
└─────────────────┘                  └─────────────────┘                  └─────────────────┘
```

**flow:**
1. client establishes websocket connection to server with tunnel id
2. server registers the tunnel and creates subdomain routing
3. external requests to `<tunnel-id>.server:port` are routed to the client
4. client forwards requests to local service and returns responses

## configuration options ⚙️

**server options:**
- `-port`: server port (default: 8080)
- `version`: show version information

**client options:**
- `http [address:port | port]`: local address to tunnel (positional argument)
- `--server`: funnel server url (default: http://localhost:8080)
- `--id`: custom tunnel id (auto-generated if not provided)
- `version`: show version information

## contributing 🤝

contributions are welcome! please follow these steps:

1. fork the repository
2. create a new branch: `git checkout -b feature/your-feature-name`
3. set up development environment: `make dev-setup`
4. make changes and test: `make build && make test`
5. format and lint: `make fmt && make lint`
6. commit your changes: `git commit -m "add your feature description"`
7. push to your fork: `git push origin feature/your-feature-name`
8. create a pull request

## license 📄

this project is licensed under the [mit license](./LICENSE.md)