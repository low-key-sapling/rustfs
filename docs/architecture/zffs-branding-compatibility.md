# ZfFS Branding and Compatibility Contract

## Purpose

ZfFS is the product-facing distribution of the RustFS object-storage engine. The product layer changes names, packaging, configuration entry points, and operator documentation without changing the storage and protocol contracts that make an existing RustFS deployment readable and operable.

The release identity is intentionally split:

- Product version: read from `ZFFS_VERSION` at build time.
- Upstream engine version: read from the RustFS workspace package version.
- Runtime identity: `zffs <product-version>` plus the RustFS base version, build target, Rust toolchain, and Git revision.

The initial product release is ZfFS `1.0.0`, based on RustFS `1.0.0-beta.12`.

## Product Layer

The following surfaces use the ZfFS identity:

- the `zffs` server binary and CLI help;
- product version and diagnostics output;
- Docker and native-package build entry points;
- deployment scripts and user-facing startup text;
- the versioned TOML server configuration file;
- the `ZFFS_*` environment-variable compatibility prefix;
- ZfFS single-node deployment documentation.

Product strings are centralized where practical. New product-facing code must not scatter independent ZfFS or RustFS identity literals when an existing product constant is available.

## Compatibility Core

Branding must not rename or rewrite the following compatibility contracts:

- internal Rust package, crate, and module names;
- `RUSTFS_*` canonical configuration keys;
- supported `MINIO_*` compatibility keys;
- `/rustfs/*` and `/minio/*` routes;
- `rustfs_*` Prometheus metrics;
- `x-rustfs-internal-*` and `x-minio-internal-*` metadata keys;
- `xl.meta`, `.metadata.bin`, erasure layout, bitrot, quorum, and heal behavior;
- protobuf, gRPC, and internode RPC method and field identities;
- existing data-directory and upgrade-read behavior.

This boundary allows a ZfFS binary to operate on existing compatible data without a branding migration of object metadata or on-disk files.

## Configuration Boundary

The canonical parser remains the RustFS parser. Product inputs are adapted only during process startup:

```text
explicit CLI
    |
    v
versioned TOML
    |
    v
ZFFS_* ----+
            +--> RUSTFS_* canonical parser
MINIO_* ----+
```

For a server option, precedence is:

```text
explicit CLI > TOML > environment-derived value > built-in default
```

Environment-prefix conflicts are validated before the TOML overlay is applied:

1. A supported `ZFFS_*` value is adapted to the corresponding `RUSTFS_*` key.
2. Equal ZfFS, RustFS, and MinIO values are accepted.
3. A differing ZfFS value and RustFS or supported MinIO value is fatal.
4. A differing MinIO and RustFS value keeps the canonical RustFS value and reports the compatibility conflict.
5. An unknown or malformed `ZFFS_*` key is fatal.
6. Diagnostics name keys and fields but never include secret values.

The TOML file is explicit: there is no implicit search path. Operators pass it with `zffs server --config <file>`. Schema version `1` is strict, rejects unknown fields, and is bounded to 1 MiB. Secret-bearing inline configuration requires private file permissions on Unix.

The complete operator-facing mapping and examples are maintained in [the single-node deployment runbook](../operations/zffs-single-node.md).

## Release Boundary

A server build alone does not complete the independent ZfFS release set. A distributable release binds these inputs and outputs:

- ZfFS server source revision and RustFS base revision;
- product version from `ZFFS_VERSION`;
- server binaries for supported architectures;
- container and native-package artifacts;
- Console source revision and static asset;
- `zfc` client source revision and binaries;
- checksums, signatures, SBOMs, and a release manifest.

Console branding, the `zfc` client, signing, and final multi-artifact release assembly remain separate release-engineering responsibilities. They must not be implemented by renaming compatibility-core identifiers.

## Release Verification Contract

Before publishing a ZfFS server artifact, verify at minimum:

1. `zffs --version` reports the intended product version, RustFS base, target architecture, and clean Git revision.
2. The binary starts with a schema-version `1` TOML file.
3. `/health/live` and `/health/ready` succeed after storage initialization.
4. Existing RustFS data is readable without an on-disk rewrite.
5. Existing RustFS and S3-compatible clients can access the ZfFS endpoint.
6. A rollback binary can open the same data directory when no incompatible upstream engine change was introduced.
7. The release manifest binds the server, Console, client, image, package, checksum, and source revisions.
