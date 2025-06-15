module github.com/karol-broda/funnel/client

go 1.24.4

require (
	github.com/gorilla/websocket v1.5.3
	github.com/karol-broda/funnel/shared v0.0.0-00010101000000-000000000000
)

require (
	github.com/mattn/go-colorable v0.1.13 // indirect
	github.com/mattn/go-isatty v0.0.20 // indirect
	github.com/rs/zerolog v1.34.0 // indirect
	golang.org/x/sys v0.31.0 // indirect
)

replace github.com/karol-broda/funnel/shared => ../shared
