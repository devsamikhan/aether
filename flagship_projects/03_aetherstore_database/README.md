# AetherStore: Embedded Key-Value Database with WAL

**AetherStore** is an embedded, zero-latency key-value database engine featuring Write-Ahead Logging (WAL) and ACID transactions, written entirely in pure **AETHER**.

## Highlights

- **ACID Transactions**: Transactional `begin_transaction()`, `commit()`, and `rollback()` semantics.
- **Write-Ahead Logging (WAL)**: Append-only durability ensuring crash tolerance and instant state replay.
- **Secondary Indexing**: Fast prefix/range lookups with `range_scan(prefix)`.
- **Pipeline Querying**: Natural queries using the `|>` operator:
  ```python
  user_count = db |> filter_prefix_keys("user:") |> count_records
  ```
- **Automated Checkpoints & Compaction**: Consolidates memtables to snapshots and compacts WAL storage.

## Running the Project

```bash
aether run flagship_projects/03_aetherstore_database/main.ae
# Or compile into standalone executable:
aether build flagship_projects/03_aetherstore_database/main.ae -o aetherstore.exe
```
