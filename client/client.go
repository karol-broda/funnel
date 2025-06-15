package client

import (
	"context"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"github.com/karol-broda/funnel/shared"
)

type Client struct {
	TunnelID          string
	ServerURL         string
	LocalAddr         string
	Conn              *websocket.Conn
	connMu            sync.Mutex
	lastPong          time.Time
	lastPongMu        sync.Mutex
	ongoingRequests   map[string]context.CancelFunc
	ongoingRequestsMu sync.Mutex
	requestSemaphore  chan struct{}
	outgoingMessages  chan *shared.Message
	closeOnce         sync.Once
	requestWg         sync.WaitGroup
	ctx               context.Context
	cancel            context.CancelFunc
}

func New(tunnelID, serverURL, localAddr string) *Client {
	ctx, cancel := context.WithCancel(context.Background())
	return &Client{
		TunnelID:         tunnelID,
		ServerURL:        serverURL,
		LocalAddr:        localAddr,
		ongoingRequests:  make(map[string]context.CancelFunc),
		requestSemaphore: make(chan struct{}, 128),
		outgoingMessages: make(chan *shared.Message, 100),
		lastPong:         time.Now(),
		ctx:              ctx,
		cancel:           cancel,
	}
}

func (c *Client) Close() {
	c.closeOnce.Do(func() {
		logger := shared.GetTunnelLogger("client", c.TunnelID)
		logger.Debug().Msg("closing client connection")

		c.cancel()

		if c.Conn != nil {
			c.Conn.Close()
		}

		if c.outgoingMessages != nil {
			close(c.outgoingMessages)
		}
	})
}

func (c *Client) updateLastPong() {
	c.lastPongMu.Lock()
	defer c.lastPongMu.Unlock()
	c.lastPong = time.Now()
}

func (c *Client) getLastPong() time.Time {
	c.lastPongMu.Lock()
	defer c.lastPongMu.Unlock()
	return c.lastPong
}
