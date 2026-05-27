## Why

Remote refreshes still paid deep `FindingReadModel` clones because the outer
snapshot arcs were shared but the inner maps were fully owned.

## Scope

- move active findings and governance decisions onto copy-on-write inner arcs
- preserve read-model semantics and ordering
- make cloned lanes and refreshes share inner maps until mutation

## Slices

### W182-S01 copy-on-write read model internals

Status: done

- store `active` and `decisions` as inner `Arc<BTreeMap<...>>`
- mutate through `Arc::make_mut`
- add a domain regression proving inner copy-on-write behavior

## Verification

- `cargo test -p venom-domain finding_read_model --all-features --offline`

