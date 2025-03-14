#!/bin/bash
# Various database counts for wtfutil dashboard
psql "$DATABASE_URL" -t <<SQL
SELECT RPAD('Pairs total  : ', 20) || LPAD(TO_CHAR(COUNT(*), 'FM999,999,999') || ' ', 11) || E'\n' ||
       RPAD('Pairs liquid : ', 20) || LPAD(TO_CHAR((SELECT COUNT(*) FROM pairs WHERE usd > 1000), 'FM999,999,999') || ' ', 11) || E'\n' ||
       RPAD('Factories    : ', 20) || LPAD(TO_CHAR((SELECT COUNT(*) FROM factories), 'FM999,999,999') || ' ', 11) || E'\n' ||
       RPAD('Tokens       : ', 20) || LPAD(TO_CHAR((SELECT COUNT(*) FROM tokens), 'FM999,999,999') || ' ', 11)

FROM pairs
SQL
