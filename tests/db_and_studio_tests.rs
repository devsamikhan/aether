use aether::vm::run_source;

#[test]
fn test_db_relational_sql_operations() {
    let source = r#"
# 1. Open in-memory database
let db = DB.open(":memory:")
assert(db != nil)

# 2. CREATE TABLE
db.execute("CREATE TABLE users (id INT, name TEXT, role TEXT, score INT);")
let tbls = db.tables()
assert(len(tbls) == 1)
assert(tbls[0] == "users")

# 3. INSERT INTO via SQL
db.execute("INSERT INTO users (id, name, role, score) VALUES (1, 'Sami', 'admin', 950);")
db.execute("INSERT INTO users (id, name, role, score) VALUES (2, 'Alice', 'developer', 820);")
db.execute("INSERT INTO users (id, name, role, score) VALUES (3, 'Bob', 'designer', 640);")

# 4. INSERT via helper map
db.insert("users", {"id": 4, "name": "Charlie", "role": "developer", "score": 790})

# 5. Row count
let total = db.count("users")
assert(total == 4)

# 6. SELECT with WHERE and ORDER BY
let devs = db.query("SELECT id, name, score FROM users WHERE score > 700 ORDER BY score DESC;")
assert(len(devs) == 3)
assert(devs[0]["name"] == "Sami")
assert(devs[0]["score"] == 950)
assert(devs[1]["name"] == "Alice")
assert(devs[2]["name"] == "Charlie")

# 7. UPDATE
let updated = db.execute("UPDATE users SET score = 999 WHERE name = 'Sami';")
assert(updated == 1)
let sami_row = db.query("SELECT score FROM users WHERE name = 'Sami';")
assert(sami_row[0]["score"] == 999)

# 8. DELETE
let deleted = db.execute("DELETE FROM users WHERE role = 'designer';")
assert(deleted == 1)
assert(db.count("users") == 3)

# 9. Parameterized Query with '?'
let param_rows = db.query("SELECT name FROM users WHERE score >= ? ORDER BY score ASC;", [800])
assert(len(param_rows) == 2)
assert(param_rows[0]["name"] == "Alice")
assert(param_rows[1]["name"] == "Sami")

# 10. DROP TABLE
db.execute("DROP TABLE users;")
let empty_tbls = db.tables()
assert(len(empty_tbls) == 0)
"#;

    let res = run_source(source);
    assert!(res.is_ok(), "DB relational SQL test failed: {:?}", res.err());
}

#[test]
fn test_vscode_extension_manifest_and_grammar() {
    let pkg_path = std::path::Path::new("editors/vscode/package.json");
    assert!(pkg_path.exists(), "VS Code package.json must exist");
    let pkg_content = std::fs::read_to_string(pkg_path).unwrap();
    assert!(pkg_content.contains("\"aether-language-support\""));
    assert!(pkg_content.contains("\"source.aether\""));

    let grammar_path = std::path::Path::new("editors/vscode/syntaxes/aether.tmLanguage.json");
    assert!(grammar_path.exists(), "VS Code TextMate grammar must exist");
    let grammar_content = std::fs::read_to_string(grammar_path).unwrap();
    assert!(grammar_content.contains("keyword.control.aether"));
    assert!(grammar_content.contains("support.class.aether"));
}

#[test]
fn test_flagship_06_execution() {
    let script_path = std::path::Path::new("flagship_projects/06_aetherbrain_ai_saas/main.ae");
    assert!(script_path.exists(), "Flagship 06 main.ae must exist");
    let source = std::fs::read_to_string(script_path).unwrap();
    let res = run_source(&source);
    assert!(res.is_ok(), "Flagship 06 execution failed: {:?}", res.err());
}
