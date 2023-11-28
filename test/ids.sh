cd test
sqlite3 test.db <<EOF
SELECT ROW_NUMBER() OVER (ORDER BY name) as temp_id, * FROM data;
EOF
