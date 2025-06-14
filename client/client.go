package client

import (
	"context"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

type Client struct {
	ServerURL         string
	LocalAddr         string
	TunnelID          string
	Conn              *websocket.Conn
	mu                sync.Mutex
	ongoingRequests   map[string]context.CancelFunc
	ongoingRequestsMu sync.Mutex

	requestSemaphore chan struct{}

	// connection monitoring
	lastPongReceived time.Time
	connectionHealth sync.RWMutex
}

func NewClient(serverURL, localAddr, tunnelID string) *Client {
	return &Client{
		ServerURL:        serverURL,
		LocalAddr:        localAddr,
		TunnelID:         tunnelID,
		ongoingRequests:  make(map[string]context.CancelFunc),
		requestSemaphore: make(chan struct{}, 8),
		lastPongReceived: time.Now(),
	}
}

func (c *Client) Run() {
	c.runWithReconnection()
}

func (c *Client) updateLastPong() {
	c.connectionHealth.Lock()
	c.lastPongReceived = time.Now()
	c.connectionHealth.Unlock()
}

func (c *Client) getLastPong() time.Time {
	c.connectionHealth.RLock()
	defer c.connectionHealth.RUnlock()
	return c.lastPongReceived
}
