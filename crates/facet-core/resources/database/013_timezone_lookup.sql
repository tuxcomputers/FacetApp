-- timezone_lookup
-- Every name a time zone answers to, canonical and legacy alike, against the one id it resolves to.

CREATE VIEW IF NOT EXISTS timezone_lookup AS
SELECT timezone_name, timezone_id FROM timezone
UNION ALL
SELECT timezone_alias_name AS timezone_name, timezone_id FROM timezone_alias;
