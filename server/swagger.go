package server

import (
	"net/http"
	"strings"

	"github.com/karol-broda/funnel/shared"
	httpSwagger "github.com/swaggo/http-swagger/v2"
)

func (tr *TunnelRouter) ServeSwaggerUI(w http.ResponseWriter, r *http.Request) {
	logger := shared.GetLogger("server.swagger")

	logger.Info().
		Str("method", r.Method).
		Str("path", r.URL.Path).
		Str("remote_addr", r.RemoteAddr).
		Msg("Swagger UI request received")

	path := strings.TrimPrefix(r.URL.Path, "/swagger")

	swaggerHandler := httpSwagger.Handler(
		httpSwagger.URL("/api/swagger/doc.json"),
	)

	newPath := "/swagger" + path
	r.URL.Path = newPath

	swaggerHandler.ServeHTTP(w, r)
}
