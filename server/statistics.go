package server

import (
	"sync"
	"time"
)

type TunnelStatistics struct {
	TotalRequests        int64 `json:"total_requests"`
	TotalRequestsSuccess int64 `json:"total_requests_success"`
	TotalRequestsError   int64 `json:"total_requests_error"`
	TotalBytesIn         int64 `json:"total_bytes_in"`
	TotalBytesOut        int64 `json:"total_bytes_out"`

	AverageResponseTime float64 `json:"average_response_time_ms"`
	MinResponseTime     float64 `json:"min_response_time_ms"`
	MaxResponseTime     float64 `json:"max_response_time_ms"`
	RequestsPerSecond   float64 `json:"requests_per_second"`

	ConnectionsActive int       `json:"connections_active"`
	ConnectionsTotal  int64     `json:"connections_total"`
	ReconnectCount    int64     `json:"reconnect_count"`
	LastActivity      time.Time `json:"last_activity"`

	TimeoutCount     int64 `json:"timeout_count"`
	ConnectionErrors int64 `json:"connection_errors"`
	DataErrors       int64 `json:"data_errors"`

	ClientIP  string `json:"client_ip"`
	UserAgent string `json:"user_agent,omitempty"`

	CreatedAt   time.Time `json:"created_at"`
	LastUpdated time.Time `json:"last_updated"`

	mu sync.RWMutex
}

type HistoricalDataPoint struct {
	Timestamp         time.Time `json:"timestamp"`
	RequestCount      int64     `json:"request_count"`
	BytesIn           int64     `json:"bytes_in"`
	BytesOut          int64     `json:"bytes_out"`
	ResponseTime      float64   `json:"response_time_ms"`
	ErrorCount        int64     `json:"error_count"`
	ActiveConnections int       `json:"active_connections"`
}

type TunnelHistoricalData struct {
	TunnelID   string                `json:"tunnel_id"`
	DataPoints []HistoricalDataPoint `json:"data_points"`
	MaxPoints  int                   `json:"max_points"`
	Interval   time.Duration         `json:"interval_seconds"`
	mu         sync.RWMutex
}

type ServerMetrics struct {
	TotalTunnels          int64 `json:"total_tunnels"`
	ActiveTunnels         int   `json:"active_tunnels"`
	TotalRequests         int64 `json:"total_requests"`
	TotalBytesTransferred int64 `json:"total_bytes_transferred"`

	AverageResponseTime  float64 `json:"average_response_time_ms"`
	ServerRequestsPerSec float64 `json:"server_requests_per_second"`

	MemoryUsage        int64   `json:"memory_usage_bytes"`
	CPUUsage           float64 `json:"cpu_usage_percent"`
	NetworkConnections int     `json:"network_connections"`

	TotalErrors int64   `json:"total_errors"`
	ErrorRate   float64 `json:"error_rate_percent"`

	ServerUptime string    `json:"server_uptime"`
	LastUpdated  time.Time `json:"last_updated"`

	mu sync.RWMutex
}

func NewTunnelStatistics(tunnelID, clientIP, userAgent string) *TunnelStatistics {
	now := time.Now()
	return &TunnelStatistics{
		ClientIP:          clientIP,
		UserAgent:         userAgent,
		CreatedAt:         now,
		LastUpdated:       now,
		LastActivity:      now,
		MinResponseTime:   -1,
		ConnectionsActive: 1,
		ConnectionsTotal:  1,
	}
}

func (ts *TunnelStatistics) RecordRequest(responseTimeMs float64, success bool, bytesIn, bytesOut int64) {
	ts.mu.Lock()
	defer ts.mu.Unlock()

	now := time.Now()
	ts.TotalRequests++
	ts.TotalBytesIn += bytesIn
	ts.TotalBytesOut += bytesOut
	ts.LastActivity = now
	ts.LastUpdated = now

	if success {
		ts.TotalRequestsSuccess++
	} else {
		ts.TotalRequestsError++
	}

	if ts.MinResponseTime == -1 || responseTimeMs < ts.MinResponseTime {
		ts.MinResponseTime = responseTimeMs
	}
	if responseTimeMs > ts.MaxResponseTime {
		ts.MaxResponseTime = responseTimeMs
	}

	if ts.TotalRequests == 1 {
		ts.AverageResponseTime = responseTimeMs
	} else {
		ts.AverageResponseTime = (ts.AverageResponseTime*float64(ts.TotalRequests-1) + responseTimeMs) / float64(ts.TotalRequests)
	}

	duration := now.Sub(ts.CreatedAt).Seconds()
	if duration > 0 {
		ts.RequestsPerSecond = float64(ts.TotalRequests) / duration
	}
}

func (ts *TunnelStatistics) RecordError(errorType string) {
	ts.mu.Lock()
	defer ts.mu.Unlock()

	ts.LastUpdated = time.Now()

	switch errorType {
	case "timeout":
		ts.TimeoutCount++
	case "connection":
		ts.ConnectionErrors++
	case "data":
		ts.DataErrors++
	}
}

func (ts *TunnelStatistics) RecordReconnect() {
	ts.mu.Lock()
	defer ts.mu.Unlock()

	ts.ReconnectCount++
	ts.ConnectionsTotal++
	ts.LastUpdated = time.Now()
}

func (ts *TunnelStatistics) GetSnapshot() TunnelStatistics {
	ts.mu.RLock()
	defer ts.mu.RUnlock()

	return *ts
}

func NewTunnelHistoricalData(tunnelID string, maxPoints int, interval time.Duration) *TunnelHistoricalData {
	return &TunnelHistoricalData{
		TunnelID:   tunnelID,
		DataPoints: make([]HistoricalDataPoint, 0, maxPoints),
		MaxPoints:  maxPoints,
		Interval:   interval,
	}
}

func (thd *TunnelHistoricalData) AddDataPoint(stats TunnelStatistics) {
	thd.mu.Lock()
	defer thd.mu.Unlock()

	point := HistoricalDataPoint{
		Timestamp:         time.Now(),
		RequestCount:      stats.TotalRequests,
		BytesIn:           stats.TotalBytesIn,
		BytesOut:          stats.TotalBytesOut,
		ResponseTime:      stats.AverageResponseTime,
		ErrorCount:        stats.TotalRequestsError,
		ActiveConnections: stats.ConnectionsActive,
	}

	thd.DataPoints = append(thd.DataPoints, point)

	if len(thd.DataPoints) > thd.MaxPoints {
		thd.DataPoints = thd.DataPoints[len(thd.DataPoints)-thd.MaxPoints:]
	}
}

func (thd *TunnelHistoricalData) GetDataPoints() []HistoricalDataPoint {
	thd.mu.RLock()
	defer thd.mu.RUnlock()

	result := make([]HistoricalDataPoint, len(thd.DataPoints))
	copy(result, thd.DataPoints)
	return result
}

func (thd *TunnelHistoricalData) GetDataPointsInRange(start, end time.Time) []HistoricalDataPoint {
	thd.mu.RLock()
	defer thd.mu.RUnlock()

	var result []HistoricalDataPoint
	for _, point := range thd.DataPoints {
		if point.Timestamp.After(start) && point.Timestamp.Before(end) {
			result = append(result, point)
		}
	}
	return result
}

func NewServerMetrics(startTime time.Time) *ServerMetrics {
	return &ServerMetrics{
		LastUpdated: time.Now(),
	}
}

func (sm *ServerMetrics) UpdateServerMetrics(activeTunnels int, totalRequests int64, routerStats map[string]interface{}, startTime time.Time) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	sm.ActiveTunnels = activeTunnels
	sm.TotalRequests = totalRequests
	sm.ServerUptime = time.Since(startTime).String()
	sm.LastUpdated = time.Now()

	uptime := time.Since(startTime).Seconds()
	if uptime > 0 {
		sm.ServerRequestsPerSec = float64(totalRequests) / uptime
	}

	if totalRequests > 0 {
		sm.ErrorRate = (float64(sm.TotalErrors) / float64(totalRequests)) * 100
	}
}

func (sm *ServerMetrics) GetSnapshot() ServerMetrics {
	sm.mu.RLock()
	defer sm.mu.RUnlock()

	return *sm
}
