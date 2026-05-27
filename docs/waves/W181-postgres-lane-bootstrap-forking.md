## Why

One Postgres-backed `ApiState` still paid three full durable rebuilds on open,
one per lane, even after the pool became shared.

## Scope

- bootstrap one rebuilt Postgres store
- fork runtime and publication lanes from that base state
- preserve lane independence after bootstrap while reusing initial snapshot
  arcs

## Slices

### W181-S01 forked lane bootstrap

Status: done

- add one `PostgresStore::fork_from(...)`
- open one base store in `ApiState::open_postgres(...)`
- derive the other two lane services from that bootstrapped store

## Verification

- `cargo test -p venom-api postgres_open_shares_bootstrap_snapshot_arcs_across_lanes --all-features --offline`

