# ZfFS Single-Node Binary Deployment

This runbook starts the ZfFS binary directly without registering a system service. The ZfFS product layer uses the existing RustFS configuration parser and storage engine; the configuration file is a startup input and does not change the on-disk object format.

## Prerequisites

- Use a `zffs` binary built for the host architecture.
- Create a dedicated data directory with enough free space.
- Store credentials in separate files when possible.
- Do not expose the API listener until non-default credentials are configured.

The configuration file has no implicit default path. Pass it explicitly with `--config`.

## Configuration

Create `/etc/zffs/zffs.toml`:

```toml
version = 1

[server]
volumes = ["/srv/zffs/data"]
address = "0.0.0.0:9000"
domains = ["s3.example.com"]
region = "us-east-1"

[credentials]
access_key_file = "/etc/zffs/access-key"
secret_key_file = "/etc/zffs/secret-key"

[console]
enabled = true
address = "0.0.0.0:9001"

[observability]
endpoint = "http://127.0.0.1:4318"

[tls]
path = "/etc/zffs/tls"

[kms]
enabled = false
backend = "local"
key_dir = "/var/lib/zffs/kms"

[buffer]
profile_disabled = false
profile = "GeneralPurpose"
```

Remove optional sections that are not used. An empty value is not equivalent to an omitted value and is rejected for fields that require content. Unknown sections and fields are rejected.

`observability.endpoint` and `kms.vault_address` must be HTTP(S) URLs. Do not place credentials or tokens in URL userinfo, query, or fragment components; those forms are rejected.

Schema version `1` accepts these fields:

- `server`: `volumes`, `address`, `domains`, `region`, `license`
- `credentials`: `access_key` or `access_key_file`; `secret_key` or `secret_key_file`
- `console`: `enabled`, `address`
- `observability`: `endpoint`
- `tls`: `path`
- `kms`: `enabled`, `backend`, `key_dir`, `local_master_key`, `vault_address`, `vault_token`, `vault_mount_path`, `default_key_id`, `allow_insecure_dev_defaults`
- `buffer`: `profile_disabled`, `profile`

The file must be UTF-8 and no larger than 1 MiB. If it contains an inline access key, secret key, license, KMS local master key, or Vault token, group and other permissions must be disabled. Mode `0600` or `0400` satisfies this requirement. Separate credential files should also use restrictive permissions.

## Start and verify

```bash
mkdir -p /srv/zffs/data
chmod 0700 /srv/zffs/data
chmod 0600 /etc/zffs/zffs.toml /etc/zffs/access-key /etc/zffs/secret-key
./zffs server --config /etc/zffs/zffs.toml
```

From another shell, check the health endpoints:

```bash
curl -f http://127.0.0.1:9000/health/live
curl -f http://127.0.0.1:9000/health/ready
```

Stop the foreground process with `Ctrl-C`.

## Source precedence and compatibility

For the same server option, an explicit CLI argument overrides the TOML value, and the TOML value overrides the environment-derived value. Before the file is applied, supported `ZFFS_*` and existing `MINIO_*` aliases are mapped to their canonical `RUSTFS_*` keys so the current parser remains the single implementation.

If a `ZFFS_*` value disagrees with the corresponding `RUSTFS_*` or supported `MINIO_*` value, startup fails before any partial alias mapping is applied. The error lists variable names only and never their values. Existing `RUSTFS_*` behavior remains available, and omitting `--config` preserves the previous CLI and environment startup path.

The product-prefix layer is allowlisted. It covers the server options represented by schema version `1`, the existing audited compatibility keys, and validated MQTT/webhook fields with optional instance suffixes. An unknown or misspelled `ZFFS_*` variable fails startup instead of being silently ignored. Public settings that have not yet entered the product allowlist, including other audit and notification backends, must continue to use their canonical `RUSTFS_*` names until their aliases are added with field-level validation.

Do not rename or remove internal `RUSTFS_*` settings, `rustfs_*` metrics, `/rustfs/*` or `/minio/*` compatibility routes, internal metadata keys, or storage files as part of deployment branding.
