cd test
sqlite3 test.db <<EOF
.import data.csv data --csv
EOF