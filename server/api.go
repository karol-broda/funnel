package server

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/karol-broda/funnel/server/docs"
	"github.com/karol-broda/funnel/shared"
	"github.com/swaggo/swag"
)

// APIResponse represents a standard API response structure
type APIResponse struct {
	Success bool        `json:"success" example:"true"`                  // Indicates if the request was successful
	Data    interface{} `json:"data,omitempty"`                          // Response data (present on success)
	Error   string      `json:"error,omitempty" example:"error message"` // Error message (present on failure)
}

// TunnelInfo represents tunnel information for API responses
type TunnelInfo struct {
	ID               string    `json:"id" example:"my-tunnel"`                                    // Unique tunnel identifier
	CreatedAt        time.Time `json:"created_at" example:"2025-08-08T18:30:00Z"`                 // Tunnel creation timestamp
	MessagesReceived int64     `json:"messages_received" example:"45"`                            // Number of messages received from client
	MessagesSent     int64     `json:"messages_sent" example:"45"`                                // Number of messages sent to client
	BytesReceived    int64     `json:"bytes_received" example:"12543"`                            // Total bytes received from client
	BytesSent        int64     `json:"bytes_sent" example:"98765"`                                // Total bytes sent to client
	Uptime           string    `json:"uptime" example:"1h15m30s"`                                 // Tunnel connection uptime
	Status           string    `json:"status" example:"connected" enums:"connected,disconnected"` // Current tunnel status
}

// ServerStats represents server-wide statistics
type ServerStats struct {
	ActiveTunnels int                    `json:"active_tunnels" example:"2"`     // Number of currently active tunnel connections
	TotalRequests int64                  `json:"total_requests" example:"1542"`  // Total number of HTTP requests processed
	CacheHits     int64                  `json:"cache_hits" example:"1401"`      // Number of cache hits for hostname resolution
	CacheMisses   int64                  `json:"cache_misses" example:"141"`     // Number of cache misses for hostname resolution
	CacheHitRate  string                 `json:"cache_hit_rate" example:"90.9%"` // Cache hit rate percentage
	Uptime        string                 `json:"uptime" example:"2h15m30s"`      // Server uptime duration
	RouterStats   map[string]interface{} `json:"router_stats"`                   // Detailed router statistics
}

// HealthData represents health check response data
type HealthData struct {
	Status    string `json:"status" example:"healthy"`                 // Service status
	Timestamp string `json:"timestamp" example:"2025-08-08T20:11:00Z"` // Current timestamp
	Uptime    string `json:"uptime" example:"6.4281425s"`              // Server uptime duration
}

// TunnelStats represents detailed tunnel statistics
type TunnelStats struct {
	TunnelID         string                 `json:"tunnel_id" example:"my-tunnel"`             // Tunnel identifier
	CreatedAt        time.Time              `json:"created_at" example:"2025-08-08T18:30:00Z"` // Creation timestamp
	Uptime           string                 `json:"uptime" example:"1h15m30s"`                 // Connection uptime
	MessagesReceived int64                  `json:"messages_received" example:"45"`            // Messages received count
	MessagesSent     int64                  `json:"messages_sent" example:"45"`                // Messages sent count
	BytesReceived    int64                  `json:"bytes_received" example:"12543"`            // Bytes received total
	BytesSent        int64                  `json:"bytes_sent" example:"98765"`                // Bytes sent total
	Status           string                 `json:"status" example:"connected"`                // Connection status
	ConnectionInfo   map[string]interface{} `json:"connection_info"`                           // Connection details
}

// TunnelDeleteData represents tunnel deletion response data
type TunnelDeleteData struct {
	Message  string `json:"message" example:"tunnel deleted successfully"` // Success message
	TunnelID string `json:"tunnel_id" example:"my-tunnel"`                 // Deleted tunnel ID
}

// DetailedTunnelMetrics represents comprehensive tunnel metrics
type DetailedTunnelMetrics struct {
	TunnelID             string  `json:"tunnel_id" example:"my-tunnel"`                // Tunnel identifier
	TotalRequests        int64   `json:"total_requests" example:"1542"`                // Total requests processed
	TotalRequestsSuccess int64   `json:"total_requests_success" example:"1498"`        // Successful requests
	TotalRequestsError   int64   `json:"total_requests_error" example:"44"`            // Failed requests
	TotalBytesIn         int64   `json:"total_bytes_in" example:"125430"`              // Total bytes received
	TotalBytesOut        int64   `json:"total_bytes_out" example:"987650"`             // Total bytes sent
	AverageResponseTime  float64 `json:"average_response_time_ms" example:"25.5"`      // Average response time in ms
	MinResponseTime      float64 `json:"min_response_time_ms" example:"5.2"`           // Minimum response time in ms
	MaxResponseTime      float64 `json:"max_response_time_ms" example:"150.8"`         // Maximum response time in ms
	RequestsPerSecond    float64 `json:"requests_per_second" example:"12.4"`           // Requests per second
	ConnectionsActive    int     `json:"connections_active" example:"1"`               // Active connections
	ConnectionsTotal     int64   `json:"connections_total" example:"3"`                // Total connections made
	ReconnectCount       int64   `json:"reconnect_count" example:"2"`                  // Number of reconnections
	TimeoutCount         int64   `json:"timeout_count" example:"5"`                    // Number of timeouts
	ConnectionErrors     int64   `json:"connection_errors" example:"2"`                // Connection errors
	DataErrors           int64   `json:"data_errors" example:"1"`                      // Data errors
	ClientIP             string  `json:"client_ip" example:"192.168.1.100:52341"`      // Client IP address
	UserAgent            string  `json:"user_agent" example:"FunnelClient/1.0"`        // Client user agent
	LastActivity         string  `json:"last_activity" example:"2025-08-08T20:30:00Z"` // Last activity time
	CreatedAt            string  `json:"created_at" example:"2025-08-08T18:30:00Z"`    // Creation time
}

// HistoricalMetricsResponse represents historical tunnel metrics
type HistoricalMetricsResponse struct {
	TunnelID   string                `json:"tunnel_id" example:"my-tunnel"` // Tunnel identifier
	TimeRange  string                `json:"time_range" example:"24h"`      // Time range of data
	Interval   string                `json:"interval" example:"1m"`         // Data point interval
	DataPoints []HistoricalDataPoint `json:"data_points"`                   // Historical data points
}

// ServerMetricsResponse represents comprehensive server metrics
type ServerMetricsResponse struct {
	TotalTunnels          int64   `json:"total_tunnels" example:"15"`                // Total tunnels created
	ActiveTunnels         int     `json:"active_tunnels" example:"8"`                // Currently active tunnels
	TotalRequests         int64   `json:"total_requests" example:"125430"`           // Total requests processed
	TotalBytesTransferred int64   `json:"total_bytes_transferred" example:"5670000"` // Total bytes transferred
	AverageResponseTime   float64 `json:"average_response_time_ms" example:"28.7"`   // Server average response time
	ServerRequestsPerSec  float64 `json:"server_requests_per_second" example:"45.2"` // Server-wide RPS
	MemoryUsage           int64   `json:"memory_usage_bytes" example:"67108864"`     // Memory usage in bytes
	CPUUsage              float64 `json:"cpu_usage_percent" example:"15.5"`          // CPU usage percentage
	NetworkConnections    int     `json:"network_connections" example:"12"`          // Active network connections
	TotalErrors           int64   `json:"total_errors" example:"234"`                // Total errors
	ErrorRate             float64 `json:"error_rate_percent" example:"0.18"`         // Error rate percentage
	ServerUptime          string  `json:"server_uptime" example:"5h23m15s"`          // Server uptime
}

// APIHandler handles REST API requests
type APIHandler struct {
	server    *Server
	router    *TunnelRouter
	startTime time.Time
}

// NewAPIHandler creates a new API handler
func NewAPIHandler(server *Server, router *TunnelRouter) *APIHandler {
	return &APIHandler{
		server:    server,
		router:    router,
		startTime: time.Now(),
	}
}

// HandleAPIRequest routes API requests to appropriate handlers
func (api *APIHandler) HandleAPIRequest(w http.ResponseWriter, r *http.Request) {
	logger := shared.GetLogger("server.api")

	logger.Info().
		Str("method", r.Method).
		Str("path", r.URL.Path).
		Str("remote_addr", r.RemoteAddr).
		Msg("API request received")

	// Set common headers
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.Header().Set("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
	w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

	// Handle CORS preflight
	if r.Method == "OPTIONS" {
		w.WriteHeader(http.StatusOK)
		return
	}

	// Route based on path
	path := strings.TrimPrefix(r.URL.Path, "/api")

	switch {
	case path == "/health":
		api.handleHealth(w, r)
	case path == "/server/stats":
		api.handleServerStats(w, r)
	case path == "/tunnels":
		api.handleTunnels(w, r)
	case strings.HasPrefix(path, "/tunnels/"):
		api.handleTunnelSpecific(w, r, path)
	case path == "/metrics":
		api.handleMetrics(w, r)
	case path == "/metrics/historical":
		api.handleHistoricalMetrics(w, r)

	case path == "/swagger/doc.json":
		api.ServeSwaggerJSON(w, r)
	default:
		api.writeErrorResponse(w, "endpoint not found", http.StatusNotFound)
	}
}

// handleHealth returns a health check response
//
// @Summary      Health check
// @Description  Returns the health status of the server
// @Tags         Server
// @Accept       json
// @Produce      json
// @Success      200  {object}  APIResponse{data=HealthData}  "Server is healthy"
// @Router       /health [get]
func (api *APIHandler) handleHealth(w http.ResponseWriter, r *http.Request) {
	if r.Method != "GET" {
		api.writeErrorResponse(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	health := map[string]interface{}{
		"status":    "healthy",
		"timestamp": time.Now().UTC(),
		"uptime":    time.Since(api.startTime).String(),
	}

	api.writeSuccessResponse(w, health)
}

// handleServerStats returns server-wide statistics
//
// @Summary      Get server statistics
// @Description  Returns comprehensive server-wide statistics including tunnel count, request metrics, and cache performance
// @Tags         Server
// @Accept       json
// @Produce      json
// @Success      200  {object}  APIResponse{data=ServerStats}  "Server statistics retrieved successfully"
// @Router       /server/stats [get]
func (api *APIHandler) handleServerStats(w http.ResponseWriter, r *http.Request) {
	if r.Method != "GET" {
		api.writeErrorResponse(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	api.server.TunnelsMu.RLock()
	activeTunnels := len(api.server.Tunnels)
	api.server.TunnelsMu.RUnlock()

	routerStats := api.router.GetStats()

	stats := ServerStats{
		ActiveTunnels: activeTunnels,
		TotalRequests: routerStats["total_requests"].(int64),
		CacheHits:     routerStats["cache_hits"].(int64),
		CacheMisses:   routerStats["cache_misses"].(int64),
		CacheHitRate:  routerStats["cache_hit_rate"].(string),
		Uptime:        time.Since(api.startTime).String(),
		RouterStats:   routerStats,
	}

	api.writeSuccessResponse(w, stats)
}

// handleTunnels handles requests to /api/tunnels
func (api *APIHandler) handleTunnels(w http.ResponseWriter, r *http.Request) {
	switch r.Method {
	case "GET":
		api.handleListTunnels(w, r)
	default:
		api.writeErrorResponse(w, "method not allowed", http.StatusMethodNotAllowed)
	}
}

// handleListTunnels returns a list of all active tunnels
//
// @Summary      List active tunnels
// @Description  Returns a list of all currently active tunnel connections
// @Tags         Tunnels
// @Accept       json
// @Produce      json
// @Success      200  {object}  APIResponse{data=[]TunnelInfo}  "List of active tunnels"
// @Router       /tunnels [get]
func (api *APIHandler) handleListTunnels(w http.ResponseWriter, r *http.Request) {
	api.server.TunnelsMu.RLock()
	defer api.server.TunnelsMu.RUnlock()

	tunnels := make([]TunnelInfo, 0, len(api.server.Tunnels))
	for _, tunnel := range api.server.Tunnels {
		tunnelInfo := api.getTunnelInfo(tunnel)
		tunnels = append(tunnels, tunnelInfo)
	}

	api.writeSuccessResponse(w, tunnels)
}

// handleTunnelSpecific handles requests to specific tunnel endpoints
func (api *APIHandler) handleTunnelSpecific(w http.ResponseWriter, r *http.Request, path string) {
	// Extract tunnel ID from path: /tunnels/{id} or /tunnels/{id}/stats
	pathParts := strings.Split(strings.TrimPrefix(path, "/tunnels/"), "/")
	if len(pathParts) == 0 || pathParts[0] == "" {
		api.writeErrorResponse(w, "tunnel ID required", http.StatusBadRequest)
		return
	}

	tunnelID := pathParts[0]

	// Validate tunnel ID format
	if err := shared.ValidateTunnelID(tunnelID); err != nil {
		api.writeErrorResponse(w, fmt.Sprintf("invalid tunnel ID format: %s", err.Error()), http.StatusBadRequest)
		return
	}

	tunnel, exists := api.server.GetTunnel(tunnelID)
	if !exists {
		api.writeErrorResponse(w, "tunnel not found", http.StatusNotFound)
		return
	}

	// Handle different endpoints
	if len(pathParts) == 1 {
		// /tunnels/{id}
		switch r.Method {
		case "GET":
			api.handleGetTunnel(w, r, tunnel)
		case "DELETE":
			api.handleDeleteTunnel(w, r, tunnel)
		default:
			api.writeErrorResponse(w, "method not allowed", http.StatusMethodNotAllowed)
		}
	} else if len(pathParts) == 2 && pathParts[1] == "stats" {
		// /tunnels/{id}/stats
		switch r.Method {
		case "GET":
			api.handleGetTunnelStats(w, r, tunnel)
		default:
			api.writeErrorResponse(w, "method not allowed", http.StatusMethodNotAllowed)
		}
	} else if len(pathParts) == 2 && pathParts[1] == "metrics" {
		// /tunnels/{id}/metrics
		switch r.Method {
		case "GET":
			api.handleGetTunnelMetrics(w, r, tunnel)
		default:
			api.writeErrorResponse(w, "method not allowed", http.StatusMethodNotAllowed)
		}
	} else if len(pathParts) == 2 && pathParts[1] == "historical" {
		// /tunnels/{id}/historical
		switch r.Method {
		case "GET":
			api.handleGetTunnelHistorical(w, r, tunnel)
		default:
			api.writeErrorResponse(w, "method not allowed", http.StatusMethodNotAllowed)
		}
	} else {
		api.writeErrorResponse(w, "endpoint not found", http.StatusNotFound)
	}
}

// handleGetTunnel returns information about a specific tunnel
//
// @Summary      Get tunnel details
// @Description  Returns detailed information about a specific tunnel
// @Tags         Tunnels
// @Accept       json
// @Produce      json
// @Param        tunnelId   path      string  true  "Tunnel ID (3-63 characters, lowercase letters, numbers, and hyphens only)" minlength(3) maxlength(63)
// @Success      200  {object}  APIResponse{data=TunnelInfo}  "Tunnel details retrieved successfully"
// @Failure      400  {object}  APIResponse  "Invalid tunnel ID format"
// @Failure      404  {object}  APIResponse  "Tunnel not found"
// @Router       /tunnels/{tunnelId} [get]
func (api *APIHandler) handleGetTunnel(w http.ResponseWriter, r *http.Request, tunnel *Tunnel) {
	tunnelInfo := api.getTunnelInfo(tunnel)
	api.writeSuccessResponse(w, tunnelInfo)
}

// handleDeleteTunnel forcibly closes a tunnel
//
// @Summary      Delete tunnel
// @Description  Forcibly closes a tunnel connection
// @Tags         Tunnels
// @Accept       json
// @Produce      json
// @Param        tunnelId   path      string  true  "Tunnel ID to delete" minlength(3) maxlength(63)
// @Success      200  {object}  APIResponse{data=TunnelDeleteData}  "Tunnel deleted successfully"
// @Failure      400  {object}  APIResponse  "Invalid tunnel ID format"
// @Failure      404  {object}  APIResponse  "Tunnel not found"
// @Router       /tunnels/{tunnelId} [delete]
func (api *APIHandler) handleDeleteTunnel(w http.ResponseWriter, r *http.Request, tunnel *Tunnel) {
	logger := shared.GetTunnelLogger("server.api", tunnel.ID)

	logger.Info().
		Str("remote_addr", r.RemoteAddr).
		Msg("tunnel deletion requested via API")

	// Remove the tunnel
	api.server.RemoveTunnel(tunnel.ID)

	response := map[string]interface{}{
		"message":   "tunnel deleted successfully",
		"tunnel_id": tunnel.ID,
	}

	api.writeSuccessResponse(w, response)
}

// handleGetTunnelStats returns detailed statistics for a specific tunnel
//
// @Summary      Get tunnel statistics
// @Description  Returns detailed statistics and metrics for a specific tunnel
// @Tags         Tunnels
// @Accept       json
// @Produce      json
// @Param        tunnelId   path      string  true  "Tunnel ID" minlength(3) maxlength(63)
// @Success      200  {object}  APIResponse{data=TunnelStats}  "Tunnel statistics retrieved successfully"
// @Failure      400  {object}  APIResponse  "Invalid tunnel ID format"
// @Failure      404  {object}  APIResponse  "Tunnel not found"
// @Router       /tunnels/{tunnelId}/stats [get]
func (api *APIHandler) handleGetTunnelStats(w http.ResponseWriter, r *http.Request, tunnel *Tunnel) {
	stats := map[string]interface{}{
		"tunnel_id":         tunnel.ID,
		"created_at":        tunnel.createdAt,
		"uptime":            time.Since(tunnel.createdAt).String(),
		"messages_received": tunnel.messagesReceived,
		"messages_sent":     tunnel.messagesSent,
		"bytes_received":    tunnel.bytesReceived,
		"bytes_sent":        tunnel.bytesSent,
		"status":            "connected",
		"connection_info": map[string]interface{}{
			"remote_addr": tunnel.getRemoteAddr(),
		},
	}

	api.writeSuccessResponse(w, stats)
}

// getTunnelInfo converts a Tunnel to TunnelInfo for API responses
func (api *APIHandler) getTunnelInfo(tunnel *Tunnel) TunnelInfo {
	return TunnelInfo{
		ID:               tunnel.ID,
		CreatedAt:        tunnel.createdAt,
		MessagesReceived: tunnel.messagesReceived,
		MessagesSent:     tunnel.messagesSent,
		BytesReceived:    tunnel.bytesReceived,
		BytesSent:        tunnel.bytesSent,
		Uptime:           time.Since(tunnel.createdAt).String(),
		Status:           "connected",
	}
}

// writeSuccessResponse writes a successful API response
func (api *APIHandler) writeSuccessResponse(w http.ResponseWriter, data interface{}) {
	response := APIResponse{
		Success: true,
		Data:    data,
	}

	w.WriteHeader(http.StatusOK)
	if err := json.NewEncoder(w).Encode(response); err != nil {
		logger := shared.GetLogger("server.api")
		logger.Error().Err(err).Msg("failed to encode success response")
	}
}

// writeErrorResponse writes an error API response
func (api *APIHandler) writeErrorResponse(w http.ResponseWriter, message string, statusCode int) {
	response := APIResponse{
		Success: false,
		Error:   message,
	}

	w.WriteHeader(statusCode)
	if err := json.NewEncoder(w).Encode(response); err != nil {
		logger := shared.GetLogger("server.api")
		logger.Error().Err(err).Msg("failed to encode error response")
	}
}

// ServeSwaggerJSON serves the swagger.json file for the Swagger UI
func (api *APIHandler) ServeSwaggerJSON(w http.ResponseWriter, r *http.Request) {
	if r.Method != "GET" {
		api.writeErrorResponse(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	logger := shared.GetLogger("server.api")
	logger.Info().
		Str("remote_addr", r.RemoteAddr).
		Msg("Swagger JSON requested")

	// Update swagger info with current request details
	scheme := "http"
	if r.TLS != nil {
		scheme = "https"
	}

	docs.SwaggerInfo.Host = r.Host
	docs.SwaggerInfo.Schemes = []string{scheme}
	docs.SwaggerInfo.BasePath = "/api"

	// Get the swagger specification
	doc, err := swag.ReadDoc("swagger")
	if err != nil {
		logger.Error().Err(err).Msg("failed to read swagger documentation")
		api.writeErrorResponse(w, "failed to generate swagger documentation", http.StatusInternalServerError)
		return
	}

	// Set headers for JSON response
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.WriteHeader(http.StatusOK)

	// Write the swagger JSON
	if _, err := w.Write([]byte(doc)); err != nil {
		logger.Error().Err(err).Msg("failed to write swagger documentation")
	}
}

// handleMetrics returns comprehensive server-wide metrics
//
// @Summary      Get comprehensive server metrics
// @Description  Returns detailed server-wide metrics including performance, resource usage, and aggregated statistics
// @Tags         Server
// @Accept       json
// @Produce      json
// @Success      200  {object}  APIResponse{data=ServerMetricsResponse}  "Comprehensive server metrics"
// @Router       /metrics [get]
func (api *APIHandler) handleMetrics(w http.ResponseWriter, r *http.Request) {
	if r.Method != "GET" {
		api.writeErrorResponse(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	api.server.TunnelsMu.RLock()
	activeTunnels := len(api.server.Tunnels)
	totalTunnels := int64(0)
	totalRequests := int64(0)
	totalBytes := int64(0)
	totalErrors := int64(0)

	// Aggregate statistics from all tunnels
	for _, tunnel := range api.server.Tunnels {
		if tunnel.statistics != nil {
			stats := tunnel.statistics.GetSnapshot()
			totalRequests += stats.TotalRequests
			totalBytes += stats.TotalBytesIn + stats.TotalBytesOut
			totalErrors += stats.TotalRequestsError
		}
		totalTunnels++
	}
	api.server.TunnelsMu.RUnlock()

	// Calculate server-wide metrics
	uptime := time.Since(api.startTime)
	serverRequestsPerSec := float64(0)
	if uptime.Seconds() > 0 {
		serverRequestsPerSec = float64(totalRequests) / uptime.Seconds()
	}

	errorRate := float64(0)
	if totalRequests > 0 {
		errorRate = (float64(totalErrors) / float64(totalRequests)) * 100
	}

	metrics := ServerMetricsResponse{
		TotalTunnels:          totalTunnels,
		ActiveTunnels:         activeTunnels,
		TotalRequests:         totalRequests,
		TotalBytesTransferred: totalBytes,
		AverageResponseTime:   0, // Could be calculated from tunnel averages
		ServerRequestsPerSec:  serverRequestsPerSec,
		MemoryUsage:           0, // Would need runtime.ReadMemStats()
		CPUUsage:              0, // Would need system monitoring
		NetworkConnections:    activeTunnels,
		TotalErrors:           totalErrors,
		ErrorRate:             errorRate,
		ServerUptime:          uptime.String(),
	}

	api.writeSuccessResponse(w, metrics)
}

// handleHistoricalMetrics returns historical server metrics
//
// @Summary      Get historical server metrics
// @Description  Returns time-series data for server-wide metrics over a specified time range
// @Tags         Server
// @Accept       json
// @Produce      json
// @Param        range    query     string  false  "Time range (1h, 24h, 7d)" default(24h)
// @Param        interval query     string  false  "Data interval (1m, 5m, 1h)" default(5m)
// @Success      200  {object}  APIResponse{data=HistoricalMetricsResponse}  "Historical server metrics"
// @Router       /metrics/historical [get]
func (api *APIHandler) handleHistoricalMetrics(w http.ResponseWriter, r *http.Request) {
	if r.Method != "GET" {
		api.writeErrorResponse(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Parse query parameters
	timeRange := r.URL.Query().Get("range")
	if timeRange == "" {
		timeRange = "24h"
	}

	interval := r.URL.Query().Get("interval")
	if interval == "" {
		interval = "5m"
	}

	// For now, return aggregated data from all tunnels
	// In a production system, this would come from a time-series database
	response := HistoricalMetricsResponse{
		TunnelID:   "server",
		TimeRange:  timeRange,
		Interval:   interval,
		DataPoints: []HistoricalDataPoint{}, // Would be populated from historical data
	}

	api.writeSuccessResponse(w, response)
}

// handleGetTunnelMetrics returns comprehensive metrics for a specific tunnel
//
// @Summary      Get comprehensive tunnel metrics
// @Description  Returns detailed performance and usage metrics for a specific tunnel
// @Tags         Tunnels
// @Accept       json
// @Produce      json
// @Param        tunnelId   path      string  true  "Tunnel ID" minlength(3) maxlength(63)
// @Success      200  {object}  APIResponse{data=DetailedTunnelMetrics}  "Comprehensive tunnel metrics"
// @Failure      400  {object}  APIResponse  "Invalid tunnel ID format"
// @Failure      404  {object}  APIResponse  "Tunnel not found"
// @Router       /tunnels/{tunnelId}/metrics [get]
func (api *APIHandler) handleGetTunnelMetrics(w http.ResponseWriter, r *http.Request, tunnel *Tunnel) {
	if tunnel.statistics == nil {
		api.writeErrorResponse(w, "tunnel statistics not available", http.StatusInternalServerError)
		return
	}

	stats := tunnel.statistics.GetSnapshot()

	metrics := DetailedTunnelMetrics{
		TunnelID:             tunnel.ID,
		TotalRequests:        stats.TotalRequests,
		TotalRequestsSuccess: stats.TotalRequestsSuccess,
		TotalRequestsError:   stats.TotalRequestsError,
		TotalBytesIn:         stats.TotalBytesIn,
		TotalBytesOut:        stats.TotalBytesOut,
		AverageResponseTime:  stats.AverageResponseTime,
		MinResponseTime:      stats.MinResponseTime,
		MaxResponseTime:      stats.MaxResponseTime,
		RequestsPerSecond:    stats.RequestsPerSecond,
		ConnectionsActive:    stats.ConnectionsActive,
		ConnectionsTotal:     stats.ConnectionsTotal,
		ReconnectCount:       stats.ReconnectCount,
		TimeoutCount:         stats.TimeoutCount,
		ConnectionErrors:     stats.ConnectionErrors,
		DataErrors:           stats.DataErrors,
		ClientIP:             stats.ClientIP,
		UserAgent:            stats.UserAgent,
		LastActivity:         stats.LastActivity.Format(time.RFC3339),
		CreatedAt:            stats.CreatedAt.Format(time.RFC3339),
	}

	api.writeSuccessResponse(w, metrics)
}

// handleGetTunnelHistorical returns historical data for a specific tunnel
//
// @Summary      Get tunnel historical data
// @Description  Returns time-series data for a specific tunnel over a specified time range
// @Tags         Tunnels
// @Accept       json
// @Produce      json
// @Param        tunnelId path      string  true  "Tunnel ID" minlength(3) maxlength(63)
// @Param        range    query     string  false  "Time range (1h, 24h, 7d)" default(24h)
// @Param        start    query     string  false  "Start time (RFC3339 format)"
// @Param        end      query     string  false  "End time (RFC3339 format)"
// @Success      200  {object}  APIResponse{data=HistoricalMetricsResponse}  "Tunnel historical data"
// @Failure      400  {object}  APIResponse  "Invalid parameters"
// @Failure      404  {object}  APIResponse  "Tunnel not found"
// @Router       /tunnels/{tunnelId}/historical [get]
func (api *APIHandler) handleGetTunnelHistorical(w http.ResponseWriter, r *http.Request, tunnel *Tunnel) {
	if tunnel.historicalData == nil {
		api.writeErrorResponse(w, "tunnel historical data not available", http.StatusInternalServerError)
		return
	}

	// Parse query parameters
	timeRange := r.URL.Query().Get("range")
	if timeRange == "" {
		timeRange = "24h"
	}

	startTimeStr := r.URL.Query().Get("start")
	endTimeStr := r.URL.Query().Get("end")

	var dataPoints []HistoricalDataPoint

	if startTimeStr != "" && endTimeStr != "" {
		// Use specific time range
		startTime, err1 := time.Parse(time.RFC3339, startTimeStr)
		endTime, err2 := time.Parse(time.RFC3339, endTimeStr)

		if err1 != nil || err2 != nil {
			api.writeErrorResponse(w, "invalid time format, use RFC3339", http.StatusBadRequest)
			return
		}

		dataPoints = tunnel.historicalData.GetDataPointsInRange(startTime, endTime)
	} else {
		// Use all available data
		dataPoints = tunnel.historicalData.GetDataPoints()
	}

	response := HistoricalMetricsResponse{
		TunnelID:   tunnel.ID,
		TimeRange:  timeRange,
		Interval:   tunnel.historicalData.Interval.String(),
		DataPoints: dataPoints,
	}

	api.writeSuccessResponse(w, response)
}
