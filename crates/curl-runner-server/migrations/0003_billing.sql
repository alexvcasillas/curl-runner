-- Stripe billing. All nullable: a team without a subscription has none of it.
ALTER TABLE teams
    ADD COLUMN stripe_customer_id text UNIQUE,
    ADD COLUMN stripe_subscription_id text UNIQUE,
    ADD COLUMN billing_interval text CHECK (billing_interval IN ('monthly', 'annual')),
    -- Stripe's subscription status, or null.
    ADD COLUMN billing_status text,
    ADD COLUMN past_due_since timestamptz,
    ADD COLUMN seats_synced int,
    ADD COLUMN cancel_at timestamptz,
    ADD COLUMN current_period_end timestamptz;

-- Webhook events already handled, so a redelivery is a no-op.
CREATE TABLE stripe_events (
    id          text PRIMARY KEY,
    type        text NOT NULL,
    received_at timestamptz NOT NULL
);
