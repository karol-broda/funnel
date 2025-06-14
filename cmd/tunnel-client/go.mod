module github.com/karol-broda/go-tunnel-proxy/cmd/tunnel-client

go 1.21

require (
	github.com/karol-broda/go-tunnel-proxy/client v0.0.0-00010101000000-000000000000
	github.com/karol-broda/go-tunnel-proxy/shared v0.0.0-00010101000000-000000000000
	github.com/karol-broda/go-tunnel-proxy/version v0.0.0-00010101000000-000000000000
)

require (
	github.com/gorilla/websocket v1.5.3 // indirect
	github.com/mattn/go-colorable v0.1.13 // indirect
	github.com/mattn/go-isatty v0.0.19 // indirect
	github.com/rs/zerolog v1.34.0 // indirect
	golang.org/x/sys v0.12.0 // indirect
)

replace github.com/karol-broda/go-tunnel-proxy/client => ../../client

replace github.com/karol-broda/go-tunnel-proxy/shared => ../../shared

replace github.com/karol-broda/go-tunnel-proxy/version => ../../version
