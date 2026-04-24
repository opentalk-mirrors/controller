-- Add the column (nullable first, so existing rows don't violate a NOT NULL constraint)
ALTER TABLE event_dates 
    ADD COLUMN duration_secs int;

-- Populate from event_recurrences where a match exists
UPDATE event_dates
    SET duration_secs = event_recurrences.duration_secs
    FROM event_recurrences
    WHERE event_recurrences.event_id = event_dates.event_id; 

-- Drop the duration_secs from event_recurrences
ALTER TABLE event_recurrences
    DROP COLUMN duration_secs;

-- Fall back to computing from starts_at / ends_at where still NULL
UPDATE event_dates
    SET duration_secs = extract(epoch FROM (ends_at - starts_at))::int
    WHERE duration_secs IS null;

-- Now that every row is populated, enforce NOT NULL
ALTER TABLE event_dates 
    ALTER COLUMN duration_secs SET NOT null;

