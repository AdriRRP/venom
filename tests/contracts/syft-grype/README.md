# Syft + Grype Fixtures

- target image: `index.docker.io/library/alpine@sha256:48b0309ca019d89d40f670aa1bc06e426dc0931948452e8491e3d65087abc07d`
- syft image: `ghcr.io/anchore/syft:v1.44.0`
- grype image: `ghcr.io/anchore/grype:v0.112.0`
- syft schema: `16.1.3`
- grype db built: `2026-05-14T07:20:27Z`

Rules:

- keep fixtures generated from official scanner releases
- prefer supported target images to avoid EOL alert noise in contract fixtures
- update this file when the committed fixture corpus changes materially
