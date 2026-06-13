-- Stores every transaction we see from the program
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    signature TEXT NOT NULL UNIQUE,   -- Solana tx signature (unique ID on-chain)
    slot BIGINT NOT NULL,             -- Block slot number
    block_time BIGINT,                -- Unix timestamp
    fee BIGINT,                       -- Transaction fee in lamports
    success BOOLEAN NOT NULL,         -- Did the transaction succeed?
    program_id TEXT NOT NULL,         -- Which program was called
    raw_data JSONB,                   -- Full raw transaction (for debugging)
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Stores each instruction inside a transaction
CREATE TABLE IF NOT EXISTS instructions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID REFERENCES transactions(id) ON DELETE CASCADE,
    instruction_index INT NOT NULL,   -- Position in the transaction
    instruction_type TEXT,            -- e.g. "swap", "deposit", "withdraw"
    accounts JSONB,                   -- All account pubkeys involved
    data JSONB,                       -- Decoded instruction arguments
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Stores accounts we seen interact with the program
CREATE TABLE IF NOT EXISTS accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pubkey TEXT NOT NULL UNIQUE,      -- Solana wallet/account address
    first_seen_slot BIGINT,
    last_seen_slot BIGINT,
    transaction_count INT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Stores specific decoded events (e.g. a Raydium swap event)
CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID REFERENCES transactions(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,         -- e.g. "swap", "add_liquidity"
    slot BIGINT NOT NULL,
    block_time BIGINT,
    data JSONB NOT NULL,              -- The actual event fields
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Tracks the last slot we processed , resume after crashes
CREATE TABLE IF NOT EXISTS slot_checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    program_id TEXT NOT NULL UNIQUE,
    last_slot BIGINT NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for fast queries
CREATE INDEX IF NOT EXISTS idx_transactions_slot ON transactions(slot);
CREATE INDEX IF NOT EXISTS idx_transactions_block_time ON transactions(block_time);
CREATE INDEX IF NOT EXISTS idx_transactions_program_id ON transactions(program_id);
CREATE INDEX IF NOT EXISTS idx_instructions_transaction_id ON instructions(transaction_id);
CREATE INDEX IF NOT EXISTS idx_instructions_type ON instructions(instruction_type);
CREATE INDEX IF NOT EXISTS idx_events_slot ON events(slot);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);