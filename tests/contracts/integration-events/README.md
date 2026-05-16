# Integration Event Contract

This directory owns the technical contract for VENOM integration-event publication.

The first implementation must satisfy these observable rules:

1. one durable business change that should be externally visible creates exactly one canonical pending outbox record in the same durable write path
2. pending outbox records survive process restart and durable-state reload
3. publication order is stable by durable event identity
4. successful publication marks the outbox record published explicitly
5. failed publication leaves the outbox record unpublished and retryable
6. external delivery semantics are at-least-once, so event identity must be stable and idempotent
7. publication runs in bounded batches; no unbounded queue growth or per-event task fan-out on the durable path

The first concrete contract checks should cover at least:

- provider report ingestion emits one pending finding-change integration event
- durable scan-command completion emits one pending scan-command integration event
- replay after restart does not lose unpublished integration events
- republishing the same pending event is detectable and explicit rather than silently hidden
