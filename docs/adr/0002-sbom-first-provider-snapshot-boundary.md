# 0002. SBOM-first provider snapshot boundary

## Status

`Accepted`

## Context

VENOM needs real findings early, but it must not become a thin wrapper around one scanner or provider.

If provider-specific webhook payloads or delta semantics leak into the core domain, reliability and portability both degrade:

- different providers disagree on identifiers, severity shapes, and lifecycle semantics
- "finding discovered" or "finding withdrawn" events from outside the core are hard to replay and verify deterministically
- fresh live intelligence conflicts with reproducible tests unless the boundary is explicit

## Decision

Use a provider boundary with these rules:

- providers report a complete snapshot for one component and one immutable artifact at one observation time
- the first real provider path is SBOM-first, using `Syft` to catalog and `Grype` to evaluate vulnerabilities
- the canonical subject identity is an immutable artifact such as an image digest or SBOM digest, not a mutable tag
- VENOM derives discovery, repetition, change, and withdrawal semantics from successive canonical snapshots
- keep two verification lanes:
  - deterministic: fixed artifact digests, fixed scanner versions, frozen vulnerability knowledge when reproducibility matters
  - live: current vulnerability knowledge and real local infrastructure when compatibility and freshness matter

## Consequences

- VENOM stays provider-agnostic while still using real scanners from the start
- `Syft + Grype` can be replaced or complemented later by `Trivy`, commercial feeds, or other adapters without reshaping the core model
- acceptance features can stay domain-readable because provider-specific payloads are normalized at the boundary
- withdrawal and deduplication logic remain testable inside VENOM instead of being outsourced to provider event semantics
