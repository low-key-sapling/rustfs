// Copyright 2024 RustFS Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::cli::ServerOpts;
use clap::{ArgMatches, Command, parser::ValueSource};
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use thiserror::Error;

const CONFIG_SCHEMA_VERSION: u32 = 1;
const MAX_CONFIG_FILE_SIZE: usize = 1024 * 1024;
const MAX_CONFIG_FILE_SIZE_U64: u64 = 1024 * 1024;

#[derive(Debug, Error)]
pub(super) enum ConfigFileError {
    #[error("failed to open configuration file {path}: {source}")]
    Open { path: PathBuf, source: std::io::Error },
    #[error("failed to read configuration file {path}: {source}")]
    Read { path: PathBuf, source: std::io::Error },
    #[error("configuration file {path} exceeds the {MAX_CONFIG_FILE_SIZE}-byte limit")]
    TooLarge { path: PathBuf },
    #[error("configuration file {path} is not valid UTF-8")]
    InvalidUtf8 { path: PathBuf },
    #[error("configuration file {path} has invalid TOML syntax, field names, or value types")]
    InvalidSchema { path: PathBuf },
    #[error("configuration file {path} uses unsupported schema version {version}; expected version {CONFIG_SCHEMA_VERSION}")]
    UnsupportedVersion { path: PathBuf, version: u32 },
    #[error("configuration field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("configuration field {field} contains an unsupported NUL character")]
    ContainsNul { field: &'static str },
    #[error("configuration field {field} must be an HTTP(S) URL without userinfo, query, or fragment components")]
    InvalidEndpoint { field: &'static str },
    #[error("configuration fields {first} and {second} cannot be used together")]
    ConflictingFields { first: &'static str, second: &'static str },
    #[cfg(unix)]
    #[error(
        "configuration file {path} contains inline secrets and must not be accessible by group or other users; use mode 0600"
    )]
    InsecurePermissions { path: PathBuf },
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UnifiedConfig {
    version: u32,
    server: Option<ServerConfig>,
    credentials: Option<CredentialsConfig>,
    console: Option<ConsoleConfig>,
    observability: Option<ObservabilityConfig>,
    tls: Option<TlsConfig>,
    kms: Option<KmsConfig>,
    buffer: Option<BufferConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServerConfig {
    volumes: Option<Vec<String>>,
    address: Option<String>,
    domains: Option<Vec<String>>,
    region: Option<String>,
    license: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialsConfig {
    access_key: Option<String>,
    access_key_file: Option<PathBuf>,
    secret_key: Option<String>,
    secret_key_file: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConsoleConfig {
    enabled: Option<bool>,
    address: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservabilityConfig {
    endpoint: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct TlsConfig {
    path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct KmsConfig {
    enabled: Option<bool>,
    backend: Option<String>,
    key_dir: Option<PathBuf>,
    local_master_key: Option<String>,
    vault_address: Option<String>,
    vault_token: Option<String>,
    vault_mount_path: Option<String>,
    default_key_id: Option<String>,
    allow_insecure_dev_defaults: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct BufferConfig {
    profile_disabled: Option<bool>,
    profile: Option<String>,
}

impl UnifiedConfig {
    fn from_path(path: &Path) -> Result<Self, ConfigFileError> {
        let mut file = File::open(path).map_err(|source| ConfigFileError::Open {
            path: path.to_path_buf(),
            source,
        })?;
        let mut bytes = Vec::new();
        file.by_ref()
            .take(MAX_CONFIG_FILE_SIZE_U64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|source| ConfigFileError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        if bytes.len() > MAX_CONFIG_FILE_SIZE {
            return Err(ConfigFileError::TooLarge {
                path: path.to_path_buf(),
            });
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| ConfigFileError::InvalidUtf8 {
            path: path.to_path_buf(),
        })?;
        let config: Self = toml_edit::de::from_str(text).map_err(|_| ConfigFileError::InvalidSchema {
            path: path.to_path_buf(),
        })?;
        config.validate(path, &file)?;
        Ok(config)
    }

    fn validate(&self, path: &Path, file: &File) -> Result<(), ConfigFileError> {
        if self.version != CONFIG_SCHEMA_VERSION {
            return Err(ConfigFileError::UnsupportedVersion {
                path: path.to_path_buf(),
                version: self.version,
            });
        }

        if let Some(server) = &self.server {
            validate_values(server.volumes.as_deref(), "server.volumes")?;
            validate_optional_string(server.address.as_deref(), "server.address")?;
            validate_values(server.domains.as_deref(), "server.domains")?;
            validate_optional_string(server.region.as_deref(), "server.region")?;
            validate_optional_string(server.license.as_deref(), "server.license")?;
        }
        if let Some(credentials) = &self.credentials {
            validate_optional_string(credentials.access_key.as_deref(), "credentials.access_key")?;
            validate_optional_path(credentials.access_key_file.as_deref(), "credentials.access_key_file")?;
            validate_optional_string(credentials.secret_key.as_deref(), "credentials.secret_key")?;
            validate_optional_path(credentials.secret_key_file.as_deref(), "credentials.secret_key_file")?;
            validate_pair(
                credentials.access_key.is_some(),
                credentials.access_key_file.is_some(),
                "credentials.access_key",
                "credentials.access_key_file",
            )?;
            validate_pair(
                credentials.secret_key.is_some(),
                credentials.secret_key_file.is_some(),
                "credentials.secret_key",
                "credentials.secret_key_file",
            )?;
        }
        if let Some(console) = &self.console {
            validate_optional_string(console.address.as_deref(), "console.address")?;
        }
        if let Some(observability) = &self.observability {
            validate_http_endpoint(observability.endpoint.as_deref(), "observability.endpoint")?;
        }
        if let Some(tls) = &self.tls {
            validate_optional_path(tls.path.as_deref(), "tls.path")?;
        }
        if let Some(kms) = &self.kms {
            validate_optional_string(kms.backend.as_deref(), "kms.backend")?;
            validate_optional_path(kms.key_dir.as_deref(), "kms.key_dir")?;
            validate_optional_string(kms.local_master_key.as_deref(), "kms.local_master_key")?;
            validate_http_endpoint(kms.vault_address.as_deref(), "kms.vault_address")?;
            validate_optional_string(kms.vault_token.as_deref(), "kms.vault_token")?;
            validate_optional_string(kms.vault_mount_path.as_deref(), "kms.vault_mount_path")?;
            validate_optional_string(kms.default_key_id.as_deref(), "kms.default_key_id")?;
        }
        if let Some(buffer) = &self.buffer {
            validate_optional_string(buffer.profile.as_deref(), "buffer.profile")?;
        }

        self.validate_secret_permissions(path, file)
    }

    #[cfg(unix)]
    fn validate_secret_permissions(&self, path: &Path, file: &File) -> Result<(), ConfigFileError> {
        use std::os::unix::fs::PermissionsExt;

        let contains_inline_secret = self.server.as_ref().is_some_and(|server| server.license.is_some())
            || self
                .credentials
                .as_ref()
                .is_some_and(|credentials| credentials.access_key.is_some() || credentials.secret_key.is_some())
            || self
                .kms
                .as_ref()
                .is_some_and(|kms| kms.local_master_key.is_some() || kms.vault_token.is_some());
        if contains_inline_secret {
            let mode = file
                .metadata()
                .map_err(|source| ConfigFileError::Read {
                    path: path.to_path_buf(),
                    source,
                })?
                .permissions()
                .mode();
            if mode & 0o077 != 0 {
                return Err(ConfigFileError::InsecurePermissions {
                    path: path.to_path_buf(),
                });
            }
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn validate_secret_permissions(&self, _path: &Path, _file: &File) -> Result<(), ConfigFileError> {
        Ok(())
    }

    pub(super) fn configure_cli(&self, command: Command) -> Command {
        let overridden_args = self.overridden_arg_ids();
        let config_supplies_volumes = self.server.as_ref().is_some_and(|server| server.volumes.is_some());

        command.mut_subcommand("server", |mut server| {
            for argument_id in overridden_args {
                server = server.mut_arg(argument_id, |argument| argument.env(None::<&'static str>));
            }
            if config_supplies_volumes {
                server = server.mut_arg("volumes", |argument| argument.required(false));
            }
            server
        })
    }

    pub(super) fn apply_to_server_opts(self, opts: &mut ServerOpts, matches: &ArgMatches) {
        if let Some(server) = self.server {
            replace_if_not_cli(&mut opts.volumes, server.volumes, matches, "volumes");
            replace_if_not_cli(&mut opts.address, server.address, matches, "address");
            replace_if_not_cli(&mut opts.server_domains, server.domains, matches, "server_domains");
            replace_optional_if_not_cli(&mut opts.region, server.region, matches, "region");
            replace_optional_if_not_cli(&mut opts.license, server.license, matches, "license");
        }
        if let Some(credentials) = self.credentials {
            let access_key_from_cli = cli_supplied(matches, "access_key") || cli_supplied(matches, "access_key_file");
            if !access_key_from_cli {
                if let Some(access_key) = credentials.access_key {
                    opts.access_key = Some(access_key);
                    opts.access_key_file = None;
                } else if let Some(access_key_file) = credentials.access_key_file {
                    opts.access_key = None;
                    opts.access_key_file = Some(access_key_file);
                }
            }

            let secret_key_from_cli = cli_supplied(matches, "secret_key") || cli_supplied(matches, "secret_key_file");
            if !secret_key_from_cli {
                if let Some(secret_key) = credentials.secret_key {
                    opts.secret_key = Some(secret_key);
                    opts.secret_key_file = None;
                } else if let Some(secret_key_file) = credentials.secret_key_file {
                    opts.secret_key = None;
                    opts.secret_key_file = Some(secret_key_file);
                }
            }
        }
        if let Some(console) = self.console {
            replace_if_not_cli(&mut opts.console_enable, console.enabled, matches, "console_enable");
            replace_if_not_cli(&mut opts.console_address, console.address, matches, "console_address");
        }
        if let Some(observability) = self.observability {
            replace_if_not_cli(&mut opts.obs_endpoint, observability.endpoint, matches, "obs_endpoint");
        }
        if let Some(tls) = self.tls {
            let path = tls.path.map(|path| path.to_string_lossy().into_owned());
            replace_optional_if_not_cli(&mut opts.tls_path, path, matches, "tls_path");
        }
        if let Some(kms) = self.kms {
            replace_if_not_cli(&mut opts.kms_enable, kms.enabled, matches, "kms_enable");
            replace_if_not_cli(&mut opts.kms_backend, kms.backend, matches, "kms_backend");
            let key_dir = kms.key_dir.map(|path| path.to_string_lossy().into_owned());
            replace_optional_if_not_cli(&mut opts.kms_key_dir, key_dir, matches, "kms_key_dir");
            replace_optional_if_not_cli(&mut opts.kms_local_master_key, kms.local_master_key, matches, "kms_local_master_key");
            replace_optional_if_not_cli(&mut opts.kms_vault_address, kms.vault_address, matches, "kms_vault_address");
            replace_optional_if_not_cli(&mut opts.kms_vault_token, kms.vault_token, matches, "kms_vault_token");
            replace_optional_if_not_cli(&mut opts.kms_vault_mount_path, kms.vault_mount_path, matches, "kms_vault_mount_path");
            replace_optional_if_not_cli(&mut opts.kms_default_key_id, kms.default_key_id, matches, "kms_default_key_id");
            replace_if_not_cli(
                &mut opts.kms_allow_insecure_dev_defaults,
                kms.allow_insecure_dev_defaults,
                matches,
                "kms_allow_insecure_dev_defaults",
            );
        }
        if let Some(buffer) = self.buffer {
            replace_if_not_cli(
                &mut opts.buffer_profile_disable,
                buffer.profile_disabled,
                matches,
                "buffer_profile_disable",
            );
            replace_if_not_cli(&mut opts.buffer_profile, buffer.profile, matches, "buffer_profile");
        }
    }

    fn overridden_arg_ids(&self) -> Vec<&'static str> {
        let mut ids = Vec::new();
        if let Some(server) = &self.server {
            push_if_configured(&mut ids, "volumes", &server.volumes);
            push_if_configured(&mut ids, "address", &server.address);
            push_if_configured(&mut ids, "server_domains", &server.domains);
            push_if_configured(&mut ids, "region", &server.region);
            push_if_configured(&mut ids, "license", &server.license);
        }
        if let Some(credentials) = &self.credentials {
            if credentials.access_key.is_some() || credentials.access_key_file.is_some() {
                ids.extend(["access_key", "access_key_file"]);
            }
            if credentials.secret_key.is_some() || credentials.secret_key_file.is_some() {
                ids.extend(["secret_key", "secret_key_file"]);
            }
        }
        if let Some(console) = &self.console {
            push_if_configured(&mut ids, "console_enable", &console.enabled);
            push_if_configured(&mut ids, "console_address", &console.address);
        }
        if let Some(observability) = &self.observability {
            push_if_configured(&mut ids, "obs_endpoint", &observability.endpoint);
        }
        if let Some(tls) = &self.tls {
            push_if_configured(&mut ids, "tls_path", &tls.path);
        }
        if let Some(kms) = &self.kms {
            push_if_configured(&mut ids, "kms_enable", &kms.enabled);
            push_if_configured(&mut ids, "kms_backend", &kms.backend);
            push_if_configured(&mut ids, "kms_key_dir", &kms.key_dir);
            push_if_configured(&mut ids, "kms_local_master_key", &kms.local_master_key);
            push_if_configured(&mut ids, "kms_vault_address", &kms.vault_address);
            push_if_configured(&mut ids, "kms_vault_token", &kms.vault_token);
            push_if_configured(&mut ids, "kms_vault_mount_path", &kms.vault_mount_path);
            push_if_configured(&mut ids, "kms_default_key_id", &kms.default_key_id);
            push_if_configured(&mut ids, "kms_allow_insecure_dev_defaults", &kms.allow_insecure_dev_defaults);
        }
        if let Some(buffer) = &self.buffer {
            push_if_configured(&mut ids, "buffer_profile_disable", &buffer.profile_disabled);
            push_if_configured(&mut ids, "buffer_profile", &buffer.profile);
        }
        ids
    }
}

fn cli_supplied(matches: &ArgMatches, argument_id: &str) -> bool {
    matches.value_source(argument_id) == Some(ValueSource::CommandLine)
}

fn replace_if_not_cli<T>(target: &mut T, configured: Option<T>, matches: &ArgMatches, argument_id: &str) {
    if !cli_supplied(matches, argument_id)
        && let Some(configured) = configured
    {
        *target = configured;
    }
}

fn replace_optional_if_not_cli<T>(target: &mut Option<T>, configured: Option<T>, matches: &ArgMatches, argument_id: &str) {
    if !cli_supplied(matches, argument_id)
        && let Some(configured) = configured
    {
        *target = Some(configured);
    }
}

fn push_if_configured<T>(ids: &mut Vec<&'static str>, argument_id: &'static str, configured: &Option<T>) {
    if configured.is_some() {
        ids.push(argument_id);
    }
}

fn validate_optional_string(value: Option<&str>, field: &'static str) -> Result<(), ConfigFileError> {
    if value.is_some_and(str::is_empty) {
        return Err(ConfigFileError::EmptyField { field });
    }
    if value.is_some_and(|value| value.contains('\0')) {
        return Err(ConfigFileError::ContainsNul { field });
    }
    Ok(())
}

fn validate_optional_path(value: Option<&Path>, field: &'static str) -> Result<(), ConfigFileError> {
    if value.is_some_and(|path| path.as_os_str().is_empty()) {
        return Err(ConfigFileError::EmptyField { field });
    }
    if value.is_some_and(|path| path.as_os_str().as_encoded_bytes().contains(&0)) {
        return Err(ConfigFileError::ContainsNul { field });
    }
    Ok(())
}

fn validate_values(values: Option<&[String]>, field: &'static str) -> Result<(), ConfigFileError> {
    let Some(values) = values else {
        return Ok(());
    };
    if values.is_empty() || values.iter().any(String::is_empty) {
        return Err(ConfigFileError::EmptyField { field });
    }
    if values.iter().any(|value| value.contains('\0')) {
        return Err(ConfigFileError::ContainsNul { field });
    }
    Ok(())
}

fn validate_http_endpoint(value: Option<&str>, field: &'static str) -> Result<(), ConfigFileError> {
    validate_optional_string(value, field)?;
    let Some(value) = value else {
        return Ok(());
    };
    let endpoint = url::Url::parse(value).map_err(|_| ConfigFileError::InvalidEndpoint { field })?;
    let has_http_authority = value.get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || value.get(..8).is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"));
    if !has_http_authority
        || !matches!(endpoint.scheme(), "http" | "https")
        || endpoint.host_str().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(ConfigFileError::InvalidEndpoint { field });
    }
    Ok(())
}

fn validate_pair(first_set: bool, second_set: bool, first: &'static str, second: &'static str) -> Result<(), ConfigFileError> {
    if first_set && second_set {
        return Err(ConfigFileError::ConflictingFields { first, second });
    }
    Ok(())
}

pub(super) fn config_path_from_args(args: &[String]) -> Result<Option<PathBuf>, clap::Error> {
    if args.get(1).map(String::as_str) != Some("server")
        || args
            .iter()
            .take_while(|argument| argument.as_str() != "--")
            .any(|argument| matches!(argument.as_str(), "--help" | "-h"))
    {
        return Ok(None);
    }

    let mut path = None;
    let mut index = 2;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--" {
            break;
        }
        let candidate = if argument == "--config" {
            index += 1;
            args.get(index)
                .cloned()
                .ok_or_else(|| clap::Error::raw(clap::error::ErrorKind::InvalidValue, "--config requires a file path"))?
        } else if let Some(value) = argument.strip_prefix("--config=") {
            if value.is_empty() {
                return Err(clap::Error::raw(clap::error::ErrorKind::InvalidValue, "--config requires a file path"));
            }
            value.to_string()
        } else {
            index += 1;
            continue;
        };
        if path.replace(PathBuf::from(candidate)).is_some() {
            return Err(clap::Error::raw(
                clap::error::ErrorKind::ArgumentConflict,
                "--config can only be specified once",
            ));
        }
        index += 1;
    }
    Ok(path)
}

pub(super) fn load_config_file_from_args(args: &[String]) -> Result<Option<UnifiedConfig>, clap::Error> {
    let Some(path) = config_path_from_args(args)? else {
        return Ok(None);
    };
    let config = UnifiedConfig::from_path(&path)
        .map_err(|error| clap::Error::raw(clap::error::ErrorKind::ValueValidation, error.to_string()))?;
    Ok(Some(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Opt;
    use serial_test::serial;
    use std::io::Write;

    fn config_file(contents: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("create temporary ZfFS configuration file");
        file.write_all(contents.as_bytes())
            .expect("write temporary ZfFS configuration file");
        file.flush().expect("flush temporary ZfFS configuration file");
        file
    }

    #[test]
    #[serial]
    fn valid_config_overlays_all_server_options() {
        let file = config_file(
            r#"
version = 1

[server]
volumes = ["/data/one", "/data/two"]
address = "0.0.0.0:9000"
domains = ["s3.example.com", "objects.example.com"]
region = "eu-west-1"
license = "signed-license"

[credentials]
access_key_file = "/run/secrets/access-key"
secret_key = "inline-secret"

[console]
enabled = false
address = "127.0.0.1:9001"

[observability]
endpoint = "http://127.0.0.1:4318"

[tls]
path = "/etc/zffs/tls"

[kms]
enabled = true
backend = "local"
key_dir = "/var/lib/zffs/kms"
local_master_key = "local-master-key"
vault_address = "https://vault.example.com"
vault_token = "vault-token"
vault_mount_path = "secret"
default_key_id = "default-key"
allow_insecure_dev_defaults = false

[buffer]
profile_disabled = true
profile = "SecureStorage"
"#,
        );
        let config_path = file.path().to_string_lossy().into_owned();

        let opt = Opt::try_parse_from(["zffs", "server", "--config", config_path.as_str()])
            .expect("parse valid ZfFS configuration file");

        assert_eq!(opt.volumes, ["/data/one", "/data/two"]);
        assert_eq!(opt.address, "0.0.0.0:9000");
        assert_eq!(opt.server_domains, ["s3.example.com", "objects.example.com"]);
        assert_eq!(opt.region.as_deref(), Some("eu-west-1"));
        assert_eq!(opt.license.as_deref(), Some("signed-license"));
        assert_eq!(opt.access_key_file.as_deref(), Some(Path::new("/run/secrets/access-key")));
        assert_eq!(opt.secret_key.as_deref(), Some("inline-secret"));
        assert!(!opt.console_enable);
        assert_eq!(opt.console_address, "127.0.0.1:9001");
        assert_eq!(opt.obs_endpoint, "http://127.0.0.1:4318");
        assert_eq!(opt.tls_path.as_deref(), Some("/etc/zffs/tls"));
        assert!(opt.kms_enable);
        assert_eq!(opt.kms_backend, "local");
        assert_eq!(opt.kms_key_dir.as_deref(), Some("/var/lib/zffs/kms"));
        assert_eq!(opt.kms_local_master_key.as_deref(), Some("local-master-key"));
        assert_eq!(opt.kms_vault_address.as_deref(), Some("https://vault.example.com"));
        assert_eq!(opt.kms_vault_token.as_deref(), Some("vault-token"));
        assert_eq!(opt.kms_vault_mount_path.as_deref(), Some("secret"));
        assert_eq!(opt.kms_default_key_id.as_deref(), Some("default-key"));
        assert!(!opt.kms_allow_insecure_dev_defaults);
        assert!(opt.buffer_profile_disable);
        assert_eq!(opt.buffer_profile, "SecureStorage");
    }

    #[test]
    fn invalid_schema_error_does_not_echo_secret_values() {
        let secret = "must-not-appear-in-errors";
        let file = config_file(&format!(
            r#"
version = 1

[credentials]
unknown_secret_field = "{secret}"
"#
        ));

        let error = UnifiedConfig::from_path(file.path()).expect_err("unknown fields must be rejected");
        let message = error.to_string();

        assert!(matches!(error, ConfigFileError::InvalidSchema { .. }));
        assert!(!message.contains(secret));
        assert!(!message.contains("unknown_secret_field"));
    }

    #[test]
    fn observability_endpoint_rejects_secret_bearing_url_components_without_echoing_them() {
        for endpoint in [
            "https://collector-user:collector-token@collector.example:4318",
            "https://collector.example:4318?access_token=query-secret",
            "https://collector.example:4318/#fragment-secret",
        ] {
            let file = config_file(&format!("version = 1\n[observability]\nendpoint = {endpoint:?}\n"));

            let error = UnifiedConfig::from_path(file.path()).expect_err("secret-bearing endpoint must fail closed");
            let message = error.to_string();

            assert!(matches!(
                error,
                ConfigFileError::InvalidEndpoint {
                    field: "observability.endpoint"
                }
            ));
            for secret in ["collector-user", "collector-token", "query-secret", "fragment-secret"] {
                assert!(!message.contains(secret), "endpoint validation error leaked {secret}: {message}");
            }
        }
    }

    #[test]
    fn vault_address_rejects_secret_bearing_url_components_without_echoing_them() {
        for endpoint in [
            "https://vault-user:vault-password@vault.example:8200",
            "https://vault.example:8200?token=query-secret",
            "https://vault.example:8200/#fragment-secret",
        ] {
            let file = config_file(&format!("version = 1\n[kms]\nvault_address = {endpoint:?}\n"));

            let error = UnifiedConfig::from_path(file.path()).expect_err("secret-bearing Vault address must fail closed");
            let message = error.to_string();

            assert!(matches!(
                error,
                ConfigFileError::InvalidEndpoint {
                    field: "kms.vault_address"
                }
            ));
            for secret in ["vault-user", "vault-password", "query-secret", "fragment-secret"] {
                assert!(!message.contains(secret), "Vault address validation error leaked {secret}: {message}");
            }
        }
    }

    #[test]
    fn http_endpoints_require_supported_scheme_and_host() {
        for endpoint in ["ftp://collector.example", "http:relative-path"] {
            let file = config_file(&format!("version = 1\n[observability]\nendpoint = {endpoint:?}\n"));

            let error = UnifiedConfig::from_path(file.path()).expect_err("invalid HTTP endpoint must fail closed");

            assert!(matches!(
                error,
                ConfigFileError::InvalidEndpoint {
                    field: "observability.endpoint"
                }
            ));
        }
    }

    #[test]
    fn unknown_fields_are_rejected_in_every_config_section() {
        for contents in [
            "version = 1\nunknown_section = true\n",
            "version = 1\n[server]\nunknown_setting = true\n",
            "version = 1\n[console]\nunknown_setting = true\n",
            "version = 1\n[observability]\nunknown_setting = true\n",
            "version = 1\n[tls]\nunknown_setting = true\n",
            "version = 1\n[kms]\nunknown_setting = true\n",
            "version = 1\n[buffer]\nunknown_setting = true\n",
        ] {
            let file = config_file(contents);
            let error = UnifiedConfig::from_path(file.path()).expect_err("unknown configuration fields must fail");

            assert!(matches!(error, ConfigFileError::InvalidSchema { .. }));
        }
    }

    #[test]
    fn oversized_and_non_utf8_files_are_rejected_before_parsing() {
        let mut size_boundary = tempfile::NamedTempFile::new().expect("create size-boundary configuration file");
        let prefix = b"version = 1\n#";
        size_boundary
            .write_all(prefix)
            .expect("write size-boundary configuration prefix");
        size_boundary
            .write_all(&vec![b'x'; MAX_CONFIG_FILE_SIZE - prefix.len()])
            .expect("fill configuration to exact size limit");
        size_boundary.flush().expect("flush size-boundary configuration file");
        let parsed = UnifiedConfig::from_path(size_boundary.path()).expect("configuration at exact size limit must parse");
        assert_eq!(parsed.version, CONFIG_SCHEMA_VERSION);

        size_boundary
            .as_file()
            .set_len(MAX_CONFIG_FILE_SIZE_U64 + 1)
            .expect("extend oversized configuration file");
        let oversized_error = UnifiedConfig::from_path(size_boundary.path()).expect_err("oversized configuration must fail");
        assert!(matches!(oversized_error, ConfigFileError::TooLarge { .. }));

        let mut non_utf8 = tempfile::NamedTempFile::new().expect("create non-UTF-8 configuration file");
        non_utf8.write_all(&[0xff]).expect("write non-UTF-8 configuration file");
        non_utf8.flush().expect("flush non-UTF-8 configuration file");
        let utf8_error = UnifiedConfig::from_path(non_utf8.path()).expect_err("non-UTF-8 configuration must fail");
        assert!(matches!(utf8_error, ConfigFileError::InvalidUtf8 { .. }));
    }

    #[test]
    fn nul_characters_are_rejected_before_option_merge() {
        let file = config_file("version = 1\n");
        let string_config = UnifiedConfig {
            version: CONFIG_SCHEMA_VERSION,
            server: Some(ServerConfig {
                region: Some("unsafe\0value".to_string()),
                ..ServerConfig::default()
            }),
            ..UnifiedConfig::default()
        };

        let error = string_config
            .validate(file.path(), file.as_file())
            .expect_err("NUL-containing configuration values must fail");
        assert!(matches!(error, ConfigFileError::ContainsNul { field: "server.region" }));

        let path_config = UnifiedConfig {
            version: CONFIG_SCHEMA_VERSION,
            tls: Some(TlsConfig {
                path: Some(PathBuf::from("unsafe\0path")),
            }),
            ..UnifiedConfig::default()
        };
        let error = path_config
            .validate(file.path(), file.as_file())
            .expect_err("NUL-containing configuration paths must fail");
        assert!(matches!(error, ConfigFileError::ContainsNul { field: "tls.path" }));
    }

    #[test]
    #[serial]
    fn typed_volume_list_accepts_paths_with_spaces() {
        let file = config_file(
            r#"
version = 1

[server]
volumes = ["/srv/zffs/data set"]
"#,
        );
        let config_path = file.path().to_string_lossy().into_owned();

        let opt = Opt::try_parse_from(["zffs", "server", "--config", config_path.as_str()])
            .expect("typed TOML volumes must not inherit environment delimiters");

        assert_eq!(opt.volumes, ["/srv/zffs/data set"]);
    }

    #[test]
    fn empty_lists_and_entries_are_rejected() {
        for (contents, field) in [
            ("version = 1\n[server]\nvolumes = []\n", "server.volumes"),
            ("version = 1\n[server]\ndomains = [\"\"]\n", "server.domains"),
            ("version = 1\n[server]\nregion = \"\"\n", "server.region"),
        ] {
            let file = config_file(contents);

            let error = UnifiedConfig::from_path(file.path()).expect_err("empty list values must fail closed");

            assert!(matches!(error, ConfigFileError::EmptyField { field: rejected } if rejected == field));
        }
    }

    #[cfg(unix)]
    #[test]
    fn inline_secrets_require_private_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let file = config_file(
            r#"
version = 1

[credentials]
secret_key = "must-not-appear-in-errors"
"#,
        );
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o640)).expect("make configuration group-readable");

        let error = UnifiedConfig::from_path(file.path()).expect_err("group-readable inline secrets must fail");
        let message = error.to_string();

        assert!(matches!(error, ConfigFileError::InsecurePermissions { .. }));
        assert!(!message.contains("must-not-appear-in-errors"));
    }

    #[test]
    fn conflicting_inline_and_file_credentials_are_rejected() {
        for (contents, first, second) in [
            (
                r#"
version = 1

[credentials]
access_key = "zffs-user"
access_key_file = "/run/secrets/access-key"
"#,
                "credentials.access_key",
                "credentials.access_key_file",
            ),
            (
                r#"
version = 1

[credentials]
secret_key = "zffs-secret"
secret_key_file = "/run/secrets/secret-key"
"#,
                "credentials.secret_key",
                "credentials.secret_key_file",
            ),
        ] {
            let file = config_file(contents);
            let error = UnifiedConfig::from_path(file.path()).expect_err("conflicting credential sources must fail");

            assert!(matches!(
                error,
                ConfigFileError::ConflictingFields {
                    first: rejected_first,
                    second: rejected_second
                } if rejected_first == first && rejected_second == second
            ));
        }
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        for version in [0, 2] {
            let file = config_file(&format!("version = {version}\n"));

            let error = UnifiedConfig::from_path(file.path()).expect_err("unknown schema versions must fail closed");

            assert!(matches!(error, ConfigFileError::UnsupportedVersion { version: rejected, .. } if rejected == version));
        }
    }

    #[test]
    fn duplicate_config_arguments_are_rejected() {
        let args = vec![
            "zffs".to_string(),
            "server".to_string(),
            "--config=one.toml".to_string(),
            "--config".to_string(),
            "two.toml".to_string(),
        ];

        let error = config_path_from_args(&args).expect_err("duplicate --config must be rejected");

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    #[serial]
    fn config_overrides_environment_and_cli_overrides_config() {
        let file = config_file(
            r#"
version = 1

[server]
volumes = ["/config-volume"]
address = "127.0.0.1:9100"
"#,
        );
        let config_path = file.path().to_string_lossy().into_owned();

        temp_env::with_vars(
            [
                ("RUSTFS_VOLUMES", Some("/environment-volume")),
                ("RUSTFS_ADDRESS", Some("127.0.0.1:9000")),
            ],
            || {
                let opt = Opt::try_parse_from([
                    "zffs",
                    "server",
                    "--config",
                    config_path.as_str(),
                    "--address",
                    "127.0.0.1:9200",
                ])
                .expect("parse config file with explicit CLI override");

                assert_eq!(opt.volumes, vec!["/config-volume"]);
                assert_eq!(opt.address, "127.0.0.1:9200");
                assert_eq!(std::env::var("RUSTFS_ADDRESS").as_deref(), Ok("127.0.0.1:9000"));
            },
        );
    }

    #[test]
    #[serial]
    fn consecutive_config_parses_do_not_leak_toml_values() {
        let first = config_file(
            r#"
version = 1

[server]
volumes = ["/first-volume"]
address = "127.0.0.1:9100"
"#,
        );
        let second = config_file(
            r#"
version = 1

[server]
volumes = ["/second-volume"]
"#,
        );
        let first_path = first.path().to_string_lossy().into_owned();
        let second_path = second.path().to_string_lossy().into_owned();

        temp_env::with_vars(
            [
                ("ZFFS_ADDRESS", None::<&str>),
                ("RUSTFS_ADDRESS", None::<&str>),
                ("MINIO_ADDRESS", None::<&str>),
                ("ZFFS_VOLUMES", None::<&str>),
                ("RUSTFS_VOLUMES", None::<&str>),
                ("MINIO_VOLUMES", None::<&str>),
            ],
            || {
                let first_opt = Opt::try_parse_from(["zffs", "server", "--config", first_path.as_str()])
                    .expect("parse first ZfFS configuration file");
                let second_opt = Opt::try_parse_from(["zffs", "server", "--config", second_path.as_str()])
                    .expect("parse second ZfFS configuration file");

                assert_eq!(first_opt.address, "127.0.0.1:9100");
                assert_eq!(second_opt.address, rustfs_config::DEFAULT_ADDRESS);
                assert_eq!(second_opt.volumes, ["/second-volume"]);
                assert!(std::env::var_os("RUSTFS_ADDRESS").is_none());
            },
        );
    }

    #[test]
    #[serial]
    fn config_credential_source_replaces_environment_counterpart() {
        let file = config_file(
            r#"
version = 1

[server]
volumes = ["/data"]

[credentials]
access_key = "config-user"
"#,
        );
        let config_path = file.path().to_string_lossy().into_owned();

        temp_env::with_vars(
            [
                ("RUSTFS_VOLUMES", None),
                ("RUSTFS_ACCESS_KEY", None),
                ("RUSTFS_ACCESS_KEY_FILE", Some("/run/secrets/environment-access-key")),
            ],
            || {
                let opt = Opt::try_parse_from(["zffs", "server", "--config", config_path.as_str()])
                    .expect("config credential must replace lower-priority environment source");

                assert_eq!(opt.access_key.as_deref(), Some("config-user"));
                assert!(opt.access_key_file.is_none());
            },
        );
    }

    #[test]
    #[serial]
    fn config_disables_invalid_and_conflicting_lower_priority_environment_values() {
        let file = config_file(
            r#"
version = 1

[server]
volumes = ["/data"]

[credentials]
access_key = "config-user"

[kms]
enabled = true
"#,
        );
        let config_path = file.path().to_string_lossy().into_owned();

        temp_env::with_vars(
            [
                ("ZFFS_ACCESS_KEY", None::<&str>),
                ("ZFFS_ACCESS_KEY_FILE", None::<&str>),
                ("RUSTFS_ACCESS_KEY", Some("environment-user")),
                ("RUSTFS_ACCESS_KEY_FILE", Some("/run/secrets/environment-access-key")),
                ("MINIO_ACCESS_KEY", None::<&str>),
                ("MINIO_ACCESS_KEY_FILE", None::<&str>),
                ("ZFFS_KMS_ENABLE", None::<&str>),
                ("RUSTFS_KMS_ENABLE", Some("not-a-bool")),
                ("RUSTFS_VOLUMES", None::<&str>),
            ],
            || {
                let opt = Opt::try_parse_from(["zffs", "server", "--config", config_path.as_str()])
                    .expect("TOML must override invalid or mutually exclusive environment values");

                assert_eq!(opt.access_key.as_deref(), Some("config-user"));
                assert!(opt.access_key_file.is_none());
                assert!(opt.kms_enable);
            },
        );
    }

    #[test]
    #[serial]
    fn explicit_cli_credentials_override_opposite_config_sources() {
        let file = config_file(
            r#"
version = 1

[server]
volumes = ["/data"]

[credentials]
access_key = "config-user"
secret_key_file = "/run/secrets/config-secret-key"
"#,
        );
        let config_path = file.path().to_string_lossy().into_owned();

        temp_env::with_vars(
            [
                ("RUSTFS_ACCESS_KEY", None::<&str>),
                ("RUSTFS_ACCESS_KEY_FILE", None::<&str>),
                ("RUSTFS_SECRET_KEY", None::<&str>),
                ("RUSTFS_SECRET_KEY_FILE", None::<&str>),
                ("RUSTFS_VOLUMES", None::<&str>),
            ],
            || {
                let opt = Opt::try_parse_from([
                    "zffs",
                    "server",
                    "--config",
                    config_path.as_str(),
                    "--access-key-file",
                    "/run/secrets/cli-access-key",
                    "--secret-key=cli-secret",
                ])
                .expect("CLI credentials must override opposite TOML sources");

                assert!(opt.access_key.is_none());
                assert_eq!(opt.access_key_file.as_deref(), Some(Path::new("/run/secrets/cli-access-key")));
                assert_eq!(opt.secret_key.as_deref(), Some("cli-secret"));
                assert!(opt.secret_key_file.is_none());
            },
        );
    }

    #[test]
    #[serial]
    fn help_like_positional_after_double_dash_does_not_skip_config() {
        let file = config_file(
            r#"
version = 1

[server]
volumes = ["/config-volume"]
address = "127.0.0.1:9100"
"#,
        );
        let config_path = file.path().to_string_lossy().into_owned();

        temp_env::with_vars([("RUSTFS_ADDRESS", None::<&str>), ("RUSTFS_VOLUMES", None::<&str>)], || {
            let opt = Opt::try_parse_from(["zffs", "server", "--config", config_path.as_str(), "--", "-h"])
                .expect("-h after -- must remain a positional volume");

            assert_eq!(opt.address, "127.0.0.1:9100");
            assert_eq!(opt.volumes, vec!["-h"]);
        });
    }

    #[test]
    #[serial]
    fn product_environment_alias_reaches_existing_cli_parser() {
        temp_env::with_vars(
            [
                ("ZFFS_VOLUMES", Some("/zffs-volume")),
                ("RUSTFS_VOLUMES", None),
                ("MINIO_VOLUMES", None),
            ],
            || {
                let opt = Opt::try_parse_from(["zffs", "server"]).expect("map ZFFS_VOLUMES into existing parser");

                assert_eq!(opt.volumes, vec!["/zffs-volume"]);
            },
        );
    }

    #[test]
    #[serial]
    fn public_non_server_parser_does_not_mutate_the_process_environment() {
        temp_env::with_vars(
            [
                ("ZFFS_ADDRESS", Some("127.0.0.1:19100")),
                ("RUSTFS_ADDRESS", None),
                ("MINIO_ADDRESS", None),
            ],
            || {
                let command =
                    Opt::parse_command(["zffs", "info", "config", "--json"]).expect("parse info command with ZFFS address alias");

                assert!(matches!(command, crate::config::CommandResult::Info(_)));
                assert!(std::env::var_os("RUSTFS_ADDRESS").is_none());
            },
        );
    }

    #[test]
    #[serial]
    fn product_conflict_error_lists_keys_without_values() {
        temp_env::with_vars(
            [
                ("ZFFS_VOLUMES", Some("/sensitive-zffs-volume")),
                ("RUSTFS_VOLUMES", Some("/sensitive-rustfs-volume")),
                ("ZFFS_REGION", Some("eu-west-1")),
                ("RUSTFS_REGION", None),
            ],
            || {
                let error = match Opt::try_parse_from(["zffs", "server"]) {
                    Ok(_) => panic!("conflicting product aliases must fail"),
                    Err(error) => error,
                };
                let message = error.to_string();

                assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
                assert!(message.contains("RUSTFS_VOLUMES"));
                assert!(!message.contains("/sensitive-zffs-volume"));
                assert!(!message.contains("/sensitive-rustfs-volume"));
                assert!(std::env::var_os("RUSTFS_REGION").is_none());
            },
        );
    }

    #[test]
    #[serial]
    fn unsupported_product_environment_error_lists_key_without_value() {
        temp_env::with_vars(
            [
                ("ZFFS_AUDIT_KAFKA_ENABLE", Some("must-not-appear-in-errors")),
                ("RUSTFS_VOLUMES", Some("/data")),
            ],
            || {
                let error = match Opt::try_parse_from(["zffs", "server"]) {
                    Ok(_) => panic!("unsupported product aliases must fail closed"),
                    Err(error) => error,
                };
                let message = error.to_string();

                assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
                assert!(message.contains("ZFFS_AUDIT_KAFKA_ENABLE"));
                assert!(!message.contains("must-not-appear-in-errors"));
            },
        );
    }
}
