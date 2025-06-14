# project configuration
PROJECT_NAME := tunneling
VERSION := 0.0.1a
BUILD_DATE := $(shell date -u +"%Y-%m-%dT%H:%M:%SZ")
GIT_COMMIT := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_TAG := $(shell git describe --tags --exact-match 2>/dev/null || echo "unknown")

# build configuration
LDFLAGS := -ldflags "\
	-X github.com/karol-broda/go-tunnel-proxy/version.Version=$(VERSION) \
	-X github.com/karol-broda/go-tunnel-proxy/version.BuildDate=$(BUILD_DATE) \
	-X github.com/karol-broda/go-tunnel-proxy/version.GitCommit=$(GIT_COMMIT) \
	-X github.com/karol-broda/go-tunnel-proxy/version.GitTag=$(GIT_TAG) \
	-s -w"

# output directories
BIN_DIR := bin
DIST_DIR := dist

# build targets
CLIENT_TARGET := $(BIN_DIR)/tunnel-client
SERVER_TARGET := $(BIN_DIR)/tunnel-server

# cross-compilation targets
PLATFORMS := linux/amd64 linux/arm64 darwin/amd64 darwin/arm64 windows/amd64

.PHONY: all
all: clean build

.PHONY: help
help:
	@echo "Available targets:"
	@echo "  build          - build client and server binaries"
	@echo "  build-client   - build only client binary"
	@echo "  build-server   - build only server binary"
	@echo "  run-client     - run client with default settings"
	@echo "  run-server     - run server with default settings"
	@echo "  test           - run all tests"
	@echo "  clean          - clean build artifacts"
	@echo "  deps           - download dependencies"
	@echo "  tidy           - recursively tidy all module dependencies"
	@echo "  deps-install   - tidy and install all dependencies"
	@echo "  list-modules   - list all discovered modules"
	@echo "  fmt            - format code"
	@echo "  lint           - run linter"
	@echo "  version        - show version information"
	@echo "  release        - build release binaries for all platforms"
	@echo "  install        - install binaries to GOPATH/bin"

.PHONY: build
build: $(CLIENT_TARGET) $(SERVER_TARGET)

.PHONY: build-client
build-client: $(CLIENT_TARGET)

.PHONY: build-server
build-server: $(SERVER_TARGET)

$(CLIENT_TARGET): deps
	@mkdir -p $(BIN_DIR)
	@echo "building client..."
	go build $(LDFLAGS) -o $(CLIENT_TARGET) ./cmd/tunnel-client

$(SERVER_TARGET): deps
	@mkdir -p $(BIN_DIR)
	@echo "building server..."
	go build $(LDFLAGS) -o $(SERVER_TARGET) ./cmd/tunnel-server

.PHONY: run-client
run-client: build-client
	$(CLIENT_TARGET) -server=http://localhost:8080 -local=localhost:3000

.PHONY: run-server
run-server: build-server
	$(SERVER_TARGET) -port=8080

.PHONY: test
test:
	@echo "running tests..."
	@echo "discovering modules..."
	@for dir in $$(find . -name "go.mod" -not -path "./go.work*" -exec dirname {} \; | sort); do \
		echo "testing module in $$dir..."; \
		(cd $$dir && go test ./...); \
	done
	@echo "all modules tested successfully"

.PHONY: clean
clean:
	@echo "cleaning build artifacts..."
	rm -rf $(BIN_DIR)
	rm -rf $(DIST_DIR)
	go clean ./...

.PHONY: deps
deps:
	@echo "downloading dependencies..."
	go work sync
	go mod download

.PHONY: tidy
tidy:
	@echo "tidying dependencies recursively..."
	@echo "discovering modules..."
	@for dir in $$(find . -name "go.mod" -not -path "./go.work*" -exec dirname {} \; | sort); do \
		echo "tidying module in $$dir..."; \
		cd $$dir && go mod tidy && cd - > /dev/null; \
	done
	@echo "synchronizing workspace..."
	go work sync
	@echo "all modules tidied successfully"

.PHONY: deps-install
deps-install: tidy
	@echo "installing all dependencies..."
	go mod download
	@echo "dependencies installed successfully"

.PHONY: list-modules
list-modules:
	@echo "discovered modules:"
	@find . -name "go.mod" -not -path "./go.work*" -exec dirname {} \; | sort | sed 's/^/  /'

.PHONY: fmt
fmt:
	@echo "formatting code..."
	@echo "discovering modules..."
	@for dir in $$(find . -name "go.mod" -not -path "./go.work*" -exec dirname {} \; | sort); do \
		echo "formatting module in $$dir..."; \
		(cd $$dir && go fmt ./...); \
	done
	@echo "all modules formatted successfully"

.PHONY: lint
lint:
	@echo "running linter..."
	@if ! command -v golangci-lint >/dev/null 2>&1; then \
		echo "golangci-lint not found. installing..."; \
		go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest; \
	fi
	@echo "discovering modules..."
	@for dir in $$(find . -name "go.mod" -not -path "./go.work*" -exec dirname {} \; | sort); do \
		echo "linting module in $$dir..."; \
		(cd $$dir && golangci-lint run ./...); \
	done
	@echo "all modules linted successfully"

.PHONY: version
version:
	@echo "version: $(VERSION)"
	@echo "build date: $(BUILD_DATE)"
	@echo "git commit: $(GIT_COMMIT)"
	@echo "git tag: $(GIT_TAG)"

.PHONY: release
release: clean
	@echo "building release binaries..."
	@mkdir -p $(DIST_DIR)
	$(foreach platform,$(PLATFORMS),\
		$(eval GOOS := $(word 1,$(subst /, ,$(platform))))\
		$(eval GOARCH := $(word 2,$(subst /, ,$(platform))))\
		$(eval EXT := $(if $(filter windows,$(GOOS)),.exe,))\
		echo "building $(GOOS)/$(GOARCH)..." && \
		GOOS=$(GOOS) GOARCH=$(GOARCH) go build $(LDFLAGS) -o $(DIST_DIR)/tunnel-client-$(GOOS)-$(GOARCH)$(EXT) ./cmd/tunnel-client && \
		GOOS=$(GOOS) GOARCH=$(GOARCH) go build $(LDFLAGS) -o $(DIST_DIR)/tunnel-server-$(GOOS)-$(GOARCH)$(EXT) ./cmd/tunnel-server;\
	)
	@echo "release binaries built in $(DIST_DIR)/"

.PHONY: install
install: build
	@echo "installing binaries..."
	go install $(LDFLAGS) ./cmd/tunnel-client
	go install $(LDFLAGS) ./cmd/tunnel-server

.PHONY: dev-setup
dev-setup:
	@echo "setting up development environment..."
	go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest
	$(MAKE) deps-install
	@echo "development environment setup complete"

# legacy targets for backward compatibility
.PHONY: build-old
build-old:
	@echo "legacy build target - use 'make build' instead"
	@$(MAKE) build
