CREATE TABLE tunnel_sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tunnel_id       TEXT NOT NULL,
    client_ip       INET,
    connected_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    disconnected_at TIMESTAMPTZ,
    bytes_in        BIGINT NOT NULL DEFAULT 0,
    bytes_out       BIGINT NOT NULL DEFAULT 0,
    requests        BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_tunnel_sessions_user ON tunnel_sessions(user_id);
CREATE INDEX idx_tunnel_sessions_tunnel ON tunnel_sessions(tunnel_id);
CREATE INDEX idx_tunnel_sessions_active ON tunnel_sessions(disconnected_at)
    WHERE disconnected_at IS NULL;
