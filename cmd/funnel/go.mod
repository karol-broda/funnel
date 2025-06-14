module github.com/karol-broda/funnel/cmd/funnel

go 1.24.4

require (
	github.com/karol-broda/funnel/client v0.0.0-00010101000000-000000000000
	github.com/karol-broda/funnel/shared v0.0.0-00010101000000-000000000000
	github.com/karol-broda/funnel/version v0.0.0-00010101000000-000000000000
)

require (
	github.com/gorilla/websocket v1.5.3 // indirect
	github.com/inconshreveable/mousetrap v1.1.0 // indirect
	github.com/mattn/go-colorable v0.1.13 // indirect
	github.com/mattn/go-isatty v0.0.19 // indirect
	github.com/rs/zerolog v1.34.0 // indirect
	github.com/spf13/cobra v1.9.1
	github.com/spf13/pflag v1.0.6 // indirect
	golang.org/x/sys v0.12.0 // indirect
)

replace github.com/karol-broda/funnel/client => ../../client

replace github.com/karol-broda/funnel/shared => ../../shared

replace github.com/karol-broda/funnel/version => ../../version
