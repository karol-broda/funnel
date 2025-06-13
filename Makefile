all: build

.PHONY: build
build:
	go build -o tunnel-client client_main.go
	go build -o tunnel-server server_main.go

.PHONY: run-client
run-client:
	go run client_main.go

.PHONY: run-server
run-server:
	go run server_main.go

.PHONY: tidy
tidy:
	go mod tidy

.PHONY: install
install:
	go install
