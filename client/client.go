package client

import (
	"context"
	"sync"

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
}

func NewClient(serverURL, localAddr, tunnelID string) *Client {
	return &Client{
		ServerURL:       serverURL,
		LocalAddr:       localAddr,
		TunnelID:        tunnelID,
		ongoingRequests: make(map[string]context.CancelFunc),
	}
}

func (c *Client) Run() {
	c.runWithReconnection()
}
