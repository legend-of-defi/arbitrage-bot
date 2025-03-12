-- Separate OLAP schema for views so we don't pollute the main schema
CREATE SCHEMA IF NOT EXISTS olap;

-- Create indexes on join columns if they don't exist
CREATE INDEX IF NOT EXISTS idx_tokens_id ON public.tokens(id);
CREATE INDEX IF NOT EXISTS idx_factories_id ON public.factories(id);
CREATE INDEX IF NOT EXISTS idx_pairs_token0_id ON public.pairs(token0_id);
CREATE INDEX IF NOT EXISTS idx_pairs_token1_id ON public.pairs(token1_id);
CREATE INDEX IF NOT EXISTS idx_pairs_factory_id ON public.pairs(factory_id);

-- Create materialized view for better performance
CREATE MATERIALIZED VIEW olap.pairs AS
    SELECT
    pairs.address,
    pairs.token0_id,
    token0.symbol AS token0_symbol,
    pairs.token1_id,
    token1.symbol AS token1_symbol,
    pairs.reserve0 / POWER(10, token0.decimals) AS reserve0,
    pairs.reserve1 / POWER(10, token1.decimals) AS reserve1,
    pairs.usd
FROM public.pairs pairs
INNER JOIN public.tokens token0 ON pairs.token0_id = token0.id
INNER JOIN public.tokens token1 ON pairs.token1_id = token1.id
INNER JOIN public.factories factory ON pairs.factory_id = factory.id;

-- Create indexes on the materialized view for common query patterns
CREATE UNIQUE INDEX idx_olap_pairs_address ON olap.pairs(address);
CREATE INDEX idx_olap_pairs_token0 ON olap.pairs(token0_id);
CREATE INDEX idx_olap_pairs_token1 ON olap.pairs(token1_id);
CREATE INDEX idx_olap_pairs_usd ON olap.pairs(usd);

-- Create a function to refresh the materialized view
CREATE OR REPLACE FUNCTION refresh_olap_pairs()
RETURNS TRIGGER AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY olap.pairs;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Create triggers to refresh the materialized view when underlying data changes
CREATE TRIGGER refresh_olap_pairs_on_pairs_change
    AFTER INSERT OR UPDATE OR DELETE ON public.pairs
    FOR EACH STATEMENT
    EXECUTE FUNCTION refresh_olap_pairs();

CREATE TRIGGER refresh_olap_pairs_on_tokens_change
    AFTER INSERT OR UPDATE OR DELETE ON public.tokens
    FOR EACH STATEMENT
    EXECUTE FUNCTION refresh_olap_pairs();

CREATE TRIGGER refresh_olap_pairs_on_factories_change
    AFTER INSERT OR UPDATE OR DELETE ON public.factories
    FOR EACH STATEMENT
    EXECUTE FUNCTION refresh_olap_pairs();