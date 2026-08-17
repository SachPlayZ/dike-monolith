CREATE TABLE IF NOT EXISTS lp_fee_claim_events (
  network TEXT NOT NULL,
  event_id TEXT NOT NULL,
  pool_id BIGINT NOT NULL,
  owner TEXT NOT NULL,
  amount TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (network, event_id)
);

CREATE INDEX IF NOT EXISTS lp_fee_claim_events_owner_idx
  ON lp_fee_claim_events (network, owner, pool_id);
