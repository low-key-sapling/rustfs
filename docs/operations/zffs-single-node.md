# ZfFS Single-Node Binary Deployment

This runbook deploys the `zffs` ARM64 or AMD64 binary directly, without registering a system service. ZfFS uses the RustFS storage and S3 compatibility core; selecting the ZfFS product configuration does not rewrite the on-disk object format.

## Deployment Model

Use a versioned TOML file for normal deployments. Environment variables remain available for container orchestration, migration, and compatibility with existing RustFS or MinIO installations.

Configuration precedence for the same server option is:

```text
explicit CLI > TOML > environment-derived value > built-in default
```

The configuration file has no implicit default path. Always pass it explicitly:

```bash
./zffs server --config /etc/zffs/zffs.toml
```

## Binary Verification

Verify the artifact before deployment:

```bash
file ./zffs
sha256sum ./zffs
./zffs --version
```

The version output must identify `zffs`, the expected ZfFS product version, the RustFS base version, the host architecture, and the intended clean Git revision.

## Directories and Credentials

The commands below assume a dedicated operating-system account is already selected. They do not register a service:

```bash
install -d -m 0700 /etc/zffs
install -d -m 0700 /srv/zffs/data
install -d -m 0700 /var/lib/zffs/kms
install -m 0600 /dev/null /etc/zffs/access-key
install -m 0600 /dev/null /etc/zffs/secret-key
```

Write the deployment-specific access key and secret key into the two credential files. Each file contains only its value. Do not reuse example or default credentials, and do not expose the API listener until credentials are configured.

## Recommended TOML Configuration

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

Remove optional sections that are not used. The file must be UTF-8, no larger than 1 MiB, and use schema version `1`. Unknown sections and fields are rejected. Empty strings, empty lists, empty list entries, and NUL characters are rejected where content is required.

Set restrictive permissions:

```bash
chmod 0600 /etc/zffs/zffs.toml /etc/zffs/access-key /etc/zffs/secret-key
```

If the TOML contains an inline access key, secret key, license, KMS local master key, or Vault token, group and other permissions must be disabled. Mode `0600` or `0400` satisfies this requirement on Unix. Credential files should always use the same restriction.

`observability.endpoint` and `kms.vault_address` must be absolute HTTP(S) URLs with a host. Userinfo, query, and fragment components are rejected so credentials cannot be hidden in a logged endpoint.

## TOML Schema Version 1

| TOML field | Type | Product environment equivalent | Notes |
| --- | --- | --- | --- |
| `server.volumes` | string array | `ZFFS_VOLUMES` | TOML preserves paths containing spaces; the environment form is space-delimited. |
| `server.address` | string | `ZFFS_ADDRESS` | API bind address, for example `0.0.0.0:9000`. |
| `server.domains` | string array | `ZFFS_SERVER_DOMAINS` | Environment form is comma-delimited. |
| `server.region` | string | `ZFFS_REGION` | S3 region. |
| `server.license` | string | `ZFFS_LICENSE` | Treat as an inline secret when stored in TOML. |
| `credentials.access_key` | string | `ZFFS_ACCESS_KEY` | Mutually exclusive with `access_key_file`. |
| `credentials.access_key_file` | path | `ZFFS_ACCESS_KEY_FILE` | Recommended for direct binary deployment. |
| `credentials.secret_key` | string | `ZFFS_SECRET_KEY` | Mutually exclusive with `secret_key_file`. |
| `credentials.secret_key_file` | path | `ZFFS_SECRET_KEY_FILE` | Recommended for direct binary deployment. |
| `console.enabled` | boolean | `ZFFS_CONSOLE_ENABLE` | Enables the bundled Console endpoint. |
| `console.address` | string | `ZFFS_CONSOLE_ADDRESS` | Console bind address. |
| `observability.endpoint` | HTTP(S) URL | `ZFFS_OBS_ENDPOINT` | OTLP/HTTP base URL. |
| `tls.path` | path | `ZFFS_TLS_PATH` | TLS certificate directory. |
| `kms.enabled` | boolean | `ZFFS_KMS_ENABLE` | Enables KMS-backed server-side encryption. |
| `kms.backend` | string | `ZFFS_KMS_BACKEND` | Existing backend values include `local`, `vault`, `vault-kv2`, `vault-transit`, `static`, and `aws`. |
| `kms.key_dir` | path | `ZFFS_KMS_KEY_DIR` | Local KMS key directory. |
| `kms.local_master_key` | string | `ZFFS_KMS_LOCAL_MASTER_KEY` | Inline secret; prefer an external secret source where available. |
| `kms.vault_address` | HTTP(S) URL | `ZFFS_KMS_VAULT_ADDRESS` | Vault base address without userinfo, query, or fragment. |
| `kms.vault_token` | string | `ZFFS_KMS_VAULT_TOKEN` | Inline secret. |
| `kms.vault_mount_path` | string | `ZFFS_KMS_VAULT_MOUNT_PATH` | Vault mount path. |
| `kms.default_key_id` | string | `ZFFS_KMS_DEFAULT_KEY_ID` | Default encryption key identifier. |
| `kms.allow_insecure_dev_defaults` | boolean | `ZFFS_KMS_ALLOW_INSECURE_DEV_DEFAULTS` | Development only; do not enable in production. |
| `buffer.profile_disabled` | boolean | `ZFFS_BUFFER_PROFILE_DISABLE` | Disables adaptive buffer profiles. |
| `buffer.profile` | string | `ZFFS_BUFFER_PROFILE` | For example `GeneralPurpose` or `SecureStorage`. |

## Environment-Only Deployment

The product prefix is a startup compatibility layer. Supported variables are mapped to the existing canonical `RUSTFS_*` parser before background threads start.

A minimal environment-only launch is:

```bash
export ZFFS_VOLUMES="/srv/zffs/data"
export ZFFS_ADDRESS="0.0.0.0:9000"
export ZFFS_ACCESS_KEY_FILE="/etc/zffs/access-key"
export ZFFS_SECRET_KEY_FILE="/etc/zffs/secret-key"
export ZFFS_CONSOLE_ENABLE="true"
export ZFFS_CONSOLE_ADDRESS="0.0.0.0:9001"
./zffs server
```

Avoid exporting inline access keys, secret keys, Vault tokens, or KMS master keys when file- or secret-manager-backed configuration is available.

### Core Product Variables

The schema table above lists the TOML-backed product variables. The product allowlist also accepts these runtime compatibility settings:

| Area | Supported `ZFFS_*` suffixes |
| --- | --- |
| Root credentials | `ROOT_USER`, `ROOT_PASSWORD` |
| API and proxy | `PORT`, `API_XFF_HEADER` |
| Compression | `COMPRESS_ENABLE`, `COMPRESS_EXTENSIONS`, `COMPRESS_MIME_TYPES` |
| Storage | `DRIVE_ACTIVE_MONITORING`, `ERASURE_SET_DRIVE_COUNT`, `STORAGE_CLASS_INLINE_BLOCK`, `STORAGE_CLASS_OPTIMIZE`, `STORAGE_CLASS_RRS`, `STORAGE_CLASS_STANDARD` |
| Scanner and ILM | `SCANNER_CYCLE`, `SCANNER_SPEED`, `ILM_EXPIRATION_WORKERS` |
| OpenID | `IDENTITY_OPENID_CLAIM_NAME`, `IDENTITY_OPENID_CLAIM_PREFIX`, `IDENTITY_OPENID_CLIENT_ID`, `IDENTITY_OPENID_CLIENT_SECRET`, `IDENTITY_OPENID_CONFIG_URL`, `IDENTITY_OPENID_DISPLAY_NAME`, `IDENTITY_OPENID_ISSUER`, `IDENTITY_OPENID_REDIRECT_URI`, `IDENTITY_OPENID_SCOPES` |
| Policy plugin | `POLICY_PLUGIN_AUTH_TOKEN`, `POLICY_PLUGIN_URL` |
| Runtime compatibility | `VERSION` |

Every suffix is prefixed with `ZFFS_`. For example, `ROOT_USER` becomes `ZFFS_ROOT_USER`. Canonical `RUSTFS_*` variables remain supported for settings that are not in the product allowlist.

### MQTT and Webhook Variables

The product prefix supports these families:

```text
ZFFS_AUDIT_MQTT_<FIELD>[_<INSTANCE>]
ZFFS_AUDIT_WEBHOOK_<FIELD>[_<INSTANCE>]
ZFFS_NOTIFY_MQTT_<FIELD>[_<INSTANCE>]
ZFFS_NOTIFY_WEBHOOK_<FIELD>[_<INSTANCE>]
```

MQTT fields:

```text
BROKER ENABLE KEEP_ALIVE_INTERVAL PASSWORD QOS QUEUE_DIR QUEUE_LIMIT
RECONNECT_INTERVAL TLS_CA TLS_CLIENT_CERT TLS_CLIENT_KEY TLS_POLICY
TLS_TRUST_LEAF_AS_CA TOPIC USERNAME WS_PATH_ALLOWLIST
```

Webhook fields:

```text
AUTH_TOKEN CLIENT_CA CLIENT_CERT CLIENT_KEY ENABLE ENDPOINT QUEUE_DIR
QUEUE_LIMIT SKIP_TLS_VERIFY
```

An optional instance name starts after the field and an underscore. It must be non-empty and contain only ASCII letters, digits, or underscores. For example:

```bash
export ZFFS_NOTIFY_WEBHOOK_ENABLE_PRIMARY="true"
export ZFFS_NOTIFY_WEBHOOK_ENDPOINT_PRIMARY="https://events.example.com/zffs"
```

Unknown fields, malformed instance names, and unsupported `ZFFS_*` variables fail startup. Other audit and notification backends must use their canonical `RUSTFS_*` names until they receive an explicit product allowlist.

## Prefix Conflict Rules

For a suffix such as `REGION`, startup applies these rules:

| Inputs | Result |
| --- | --- |
| Only `ZFFS_REGION` | Adapt to the canonical RustFS parser. |
| Equal `ZFFS_REGION` and `RUSTFS_REGION` | Start normally. |
| Equal ZfFS, RustFS, and supported MinIO values | Start normally. |
| Differing `ZFFS_REGION` and `RUSTFS_REGION` | Fail before partial product alias mapping. |
| Differing `ZFFS_REGION` and supported `MINIO_REGION` | Fail before partial product alias mapping. |
| Differing `MINIO_REGION` and `RUSTFS_REGION`, with no ZfFS value | Keep the canonical RustFS value and report a compatibility warning. |
| Unknown `ZFFS_*` variable | Fail and name the unsupported key. |

Conflict and validation messages contain variable names only. They do not contain access keys, secrets, tokens, endpoint credentials, or conflicting values. Prefix conflicts are checked before TOML precedence is applied, so remove contradictory environment variables even when TOML supplies the final option.

## Start and Verify

Start the foreground process:

```bash
./zffs server --config /etc/zffs/zffs.toml
```

From another shell, bypass any configured HTTP proxy and check both health endpoints:

```bash
curl --noproxy '*' -f http://127.0.0.1:9000/health/live
curl --noproxy '*' -f http://127.0.0.1:9000/health/ready
```

The readiness response should report storage, IAM, and lock dependencies as connected and ready. Stop the foreground process with `Ctrl-C`.

## Common Failures

| Symptom | Check |
| --- | --- |
| `--config requires a file path` | Pass an explicit path after `--config`. |
| Unsupported schema version | Set top-level `version = 1`. |
| Invalid TOML schema | Remove unknown fields and correct field types; raw secret values are intentionally omitted from the error. |
| Insecure configuration permissions | Use `chmod 0600` or `0400` when inline secrets are present. |
| Conflicting ZfFS sources | Unset the named contradictory `RUSTFS_*` or `MINIO_*` variable, or make all values equal. |
| Unsupported `ZFFS_*` variable | Correct the name or use the canonical `RUSTFS_*` setting if it is outside the product allowlist. |
| Address already in use | Select an unused API or Console bind port. |
| Health check returns a proxy error | Use `curl --noproxy '*'` for the loopback endpoint. |

## Compatibility Boundary

Do not rename or remove internal `RUSTFS_*` settings, `rustfs_*` metrics, `/rustfs/*` or `/minio/*` compatibility routes, RustFS/MinIO internal metadata keys, RPC identities, or storage files as part of deployment branding. See [the ZfFS branding and compatibility contract](../architecture/zffs-branding-compatibility.md) for the complete boundary.
