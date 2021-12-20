
test:
	cargo test --release
	./tests/main.tcl

test-db:
	sqlite3 -init tests/init_test_db.sql
