**disclaimer: ❗** this tunneling solution is intended for development and testing purposes. while functional, it may not include all security features required for production environments. use at your own discretion and implement additional security measures as needed for production deployments.

# tunneling 🚇

![go](https://img.shields.io/badge/Go-%2300ADD8.svg?style=for-the-badge&logo=go&logoColor=white)

a lightweight, high-performance tunneling solution implemented in go, designed to expose local services to the internet through secure websocket connections. this tool allows developers to share local development servers, test webhooks, and demonstrate applications without complex network configuration.

## about 📖

a lightweight, high-performance tunneling solution that creates secure connections between local services and remote clients through a central server. built with go and websockets, it provides fast, reliable tunneling with automatic reconnection and comprehensive logging. perfect for development, testing, and demonstration purposes.

## getting started 🚀

1. clone the repository:
   ```bash
   git clone https://github.com/your-username/tunneling.git
   cd tunneling
   ```

2. install dependencies:
   ```bash
   go mod tidy
   ```

3. build both client and server:
   ```bash
   make build
   ```

## usage 💻

### running the tunnel server 🖥️

start the tunnel server to accept incoming tunnel connections:

```bash
./tunnel-server
```

by default, the server listens on port `8080`. you can specify a different port:

```bash
./tunnel-server -port 9000
```

### running the tunnel client 📱

connect your local service to the tunnel server:

```bash
./tunnel-client -server http://localhost:8080 -local localhost:3000
```

the client will automatically generate a tunnel id. you can also specify a custom tunnel id:

```bash
./tunnel-client -server http://localhost:8080 -local localhost:3000 -id my-custom-tunnel
```

## example usage 🎯

1. **start your local service:**
   ```bash
   python3 -m http.server 3000
   ```

2. **start the tunnel server:**
   ```bash
   ./tunnel-server -port 8080
   ```

3. **connect the tunnel client:**
   ```bash
   ./tunnel-client -server http://localhost:8080 -local localhost:3000 -id demo
   ```

4. **access your service:**
   ```bash
   curl http://demo.localhost:8080
   ```

## architecture 🏗️

the tunneling system consists of three main components:

```
┌─────────────────┐    websocket     ┌─────────────────┐    http/https    ┌─────────────────┐
│                 │ ◄──────────────► │                 │ ◄──────────────► │                 │
│  tunnel client  │                  │  tunnel server  │                  │  remote client  │
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

**client options:**
- `-server`: tunnel server url (default: http://localhost:8080)
- `-local`: local address to tunnel (default: localhost:3000)
- `-id`: custom tunnel id (auto-generated if not provided)

## contributing 🤝

contributions are welcome! please follow these steps:

1. fork the repository
2. create a new branch: `git checkout -b feature/your-feature-name`
3. make changes and test: `make build`
4. commit your changes: `git commit -m "add your feature description"`
5. push to your fork: `git push origin feature/your-feature-name`
6. create a pull request

## license 📄

this project is licensed under the [mit license](./LICENSE.md)