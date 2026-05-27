## Why

The global `single-flight` is gone, but Postgres runtime workers still took the
state/runtime consistency barrier too often, reducing throughput without
improving truth.

## Scope

- move true state registrations back onto the state lane
- let the collection-scan worker use the relaxed Postgres runtime path
- revalidate durable state inside the Postgres collection worker before acting

## Slices

### W183-S01 narrower runtime barrier surface

Status: done

- register components, tags, and context profiles through the state lane
- run collection schedule draining through the relaxed runtime mutation path
- refresh stale Postgres state inside due-collection draining before planning

## Verification

- `cargo test -p venom-api postgres_worker_loop_drains_until_idle --all-features --offline`
- `cargo test -p venom-api detached_postgres_fresh_read_promotes_the_observed_change_watermark --all-features --offline`
