use std::fs;

use rusqlite::{Connection, params};
use zeroize::{Zeroize, Zeroizing};

fn apply_synthetic_key(connection: &Connection, key: &[u8]) {
    let mut hex_key = Zeroizing::new(
        key.iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    );
    let pragma_value = Zeroizing::new(format!("x'{}'", hex_key.as_str()));
    connection
        .pragma_update(None, "key", pragma_value.as_str())
        .unwrap();
    hex_key.zeroize();
}

#[test]
fn sqlcipher_encrypts_pages_and_reopens_with_the_same_key() {
    let temp = tempfile::tempdir().unwrap();
    let database_path = temp.path().join("sqlcipher-spike.sqlite");
    let mut key = Zeroizing::new([0_u8; 32]);
    getrandom::fill(key.as_mut()).unwrap();

    {
        let connection = Connection::open(&database_path).unwrap();
        apply_synthetic_key(&connection, key.as_ref());
        let cipher_version: String = connection
            .query_row("PRAGMA cipher_version", [], |row| row.get(0))
            .unwrap();
        assert!(!cipher_version.is_empty());
        connection
            .execute(
                "CREATE TABLE spike_records (id INTEGER PRIMARY KEY, marker TEXT NOT NULL)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO spike_records (marker) VALUES (?1)",
                params!["SYNTHETIC-SQLCIPHER-MARKER"],
            )
            .unwrap();
        connection
            .pragma_update(None, "wal_checkpoint", "TRUNCATE")
            .ok();
    }

    let bytes = fs::read(&database_path).unwrap();
    assert!(!bytes.starts_with(b"SQLite format 3\0"));
    assert!(
        !bytes
            .windows(b"SYNTHETIC-SQLCIPHER-MARKER".len())
            .any(|window| window == b"SYNTHETIC-SQLCIPHER-MARKER")
    );

    let connection = Connection::open(&database_path).unwrap();
    apply_synthetic_key(&connection, key.as_ref());
    let marker: String = connection
        .query_row("SELECT marker FROM spike_records WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(marker, "SYNTHETIC-SQLCIPHER-MARKER");
    let integrity: String = connection
        .query_row("PRAGMA cipher_integrity_check", [], |row| row.get(0))
        .unwrap_or_default();
    assert!(integrity.is_empty() || integrity == "ok");
    key.zeroize();
}

#[test]
fn sqlcipher_rejects_a_wrong_key() {
    let temp = tempfile::tempdir().unwrap();
    let database_path = temp.path().join("wrong-key.sqlite");
    {
        let connection = Connection::open(&database_path).unwrap();
        apply_synthetic_key(&connection, &[7_u8; 32]);
        connection
            .execute("CREATE TABLE protected (id INTEGER PRIMARY KEY)", [])
            .unwrap();
    }
    let connection = Connection::open(&database_path).unwrap();
    apply_synthetic_key(&connection, &[9_u8; 32]);
    assert!(
        connection
            .query_row::<i64, _, _>("SELECT COUNT(*) FROM protected", [], |row| row.get(0))
            .is_err()
    );
}
