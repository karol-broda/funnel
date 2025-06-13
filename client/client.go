package client

import (
	"github.com/gorilla/websocket"
)

type Client struct {
	ServerURL string
	LocalAddr string
	TunnelID  string
	Conn      *websocket.Conn
}

func (c *Client) Run() {
	c.runWithReconnection()
}
