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

//! Parsed server options.
//!
//! This module contains the `Opt` struct which holds parsed server options
//! and methods for parsing command line arguments.

use super::Config;
use super::cli::{Cli, CommandResult, Commands, ServerOpts, default_server_opts, preprocess_args_for_legacy};
use super::config_file::{UnifiedConfig, load_config_file_from_args};
use CommandResult::Server;
use clap::{CommandFactory, FromArgMatches};
use rustfs_utils::{ExternalEnvCompatReport, build_external_env_compat_report};
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf};

/// Parsed server options. Public for tests and backward compatibility.
/// Use `Opt::parse_from` or `Config::parse()` to obtain.
#[derive(Clone)]
pub struct Opt {
    pub volumes: Vec<String>,
    pub address: String,
    pub server_domains: Vec<String>,
    pub access_key: Option<String>,
    pub access_key_file: Option<PathBuf>,
    pub secret_key: Option<String>,
    pub secret_key_file: Option<PathBuf>,
    pub console_enable: bool,
    pub console_address: String,
    pub obs_endpoint: String,
    pub tls_path: Option<String>,
    pub license: Option<String>,
    pub region: Option<String>,
    pub kms_enable: bool,
    pub kms_backend: String,
    pub kms_key_dir: Option<String>,
    pub kms_local_master_key: Option<String>,
    pub kms_vault_address: Option<String>,
    pub kms_vault_token: Option<String>,
    pub kms_vault_mount_path: Option<String>,
    pub kms_default_key_id: Option<String>,
    pub kms_allow_insecure_dev_defaults: bool,
    pub buffer_profile_disable: bool,
    pub buffer_profile: String,
}

impl Opt {
    /// Create Opt from ServerOpts
    pub(super) fn from_server_opts(o: ServerOpts) -> Self {
        Self {
            volumes: o.volumes,
            address: o.address,
            server_domains: o.server_domains,
            access_key: o.access_key,
            access_key_file: o.access_key_file,
            secret_key: o.secret_key,
            secret_key_file: o.secret_key_file,
            console_enable: o.console_enable,
            console_address: o.console_address,
            obs_endpoint: o.obs_endpoint,
            tls_path: o.tls_path,
            license: o.license,
            region: o.region,
            kms_enable: o.kms_enable,
            kms_backend: o.kms_backend,
            kms_key_dir: o.kms_key_dir,
            kms_local_master_key: o.kms_local_master_key,
            kms_vault_address: o.kms_vault_address,
            kms_vault_token: o.kms_vault_token,
            kms_vault_mount_path: o.kms_vault_mount_path,
            kms_default_key_id: o.kms_default_key_id,
            kms_allow_insecure_dev_defaults: o.kms_allow_insecure_dev_defaults,
            buffer_profile_disable: o.buffer_profile_disable,
            buffer_profile: o.buffer_profile,
        }
    }

    fn prepare_args<I, T>(args: I) -> Result<(Vec<String>, ExternalEnvCompatReport, Option<UnifiedConfig>), clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let report = build_external_env_compat_report();
        Self::prepare_args_with_report(args, report)
    }

    fn prepare_args_with_report<I, T>(
        args: I,
        report: ExternalEnvCompatReport,
    ) -> Result<(Vec<String>, ExternalEnvCompatReport, Option<UnifiedConfig>), clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let args: Vec<String> = args
            .into_iter()
            .map(|arg| arg.into().to_string_lossy().into_owned())
            .collect();
        let args = preprocess_args_for_legacy(args);
        let is_server = args.get(1).map(String::as_str) == Some("server");
        let displays_cli_output = args
            .iter()
            .take_while(|argument| argument.as_str() != "--")
            .any(|argument| matches!(argument.as_str(), "--help" | "-h" | "--version" | "-V"));
        let (report, config) = if displays_cli_output {
            (ExternalEnvCompatReport::default(), None)
        } else {
            if report.unsupported_product_count() > 0 {
                return Err(clap::Error::raw(
                    clap::error::ErrorKind::InvalidValue,
                    format!(
                        "unsupported ZFFS_ configuration variables: {}",
                        report.unsupported_product_keys.join(", ")
                    ),
                ));
            }
            if report.product_conflict_count() > 0 {
                return Err(clap::Error::raw(
                    clap::error::ErrorKind::ArgumentConflict,
                    format!("conflicting ZFFS_ configuration sources for: {}", report.product_conflict_keys.join(", ")),
                ));
            }
            let config = if is_server { load_config_file_from_args(&args)? } else { None };
            (report, config)
        };
        Ok((args, report, config))
    }

    pub(crate) fn prepare_command_with_report<I, T>(
        args: I,
        report: ExternalEnvCompatReport,
    ) -> Result<(CommandResult, ExternalEnvCompatReport), clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let (args, report, config) = Self::prepare_args_with_report(args, report)?;
        let command = Self::parse_command_prepared(args, config, &report)?;
        Ok((command, report))
    }

    fn parse_cli_prepared(
        args: Vec<String>,
        config: Option<UnifiedConfig>,
        report: &ExternalEnvCompatReport,
    ) -> Result<Cli, clap::Error> {
        let command = config
            .as_ref()
            .map_or_else(Cli::command, |config| config.configure_cli(Cli::command()));
        let command = configure_external_env_aliases(command, report);
        let mut matches = command.try_get_matches_from(args)?;
        let server_matches = matches.subcommand_matches("server").cloned();
        let mut cli = Cli::from_arg_matches_mut(&mut matches)?;

        if let (Some(config), Some(Commands::Server(opts)), Some(server_matches)) =
            (config, cli.command.as_mut(), server_matches.as_ref())
        {
            config.apply_to_server_opts(opts.as_mut(), server_matches);
        }
        Ok(cli)
    }

    fn parse_command_prepared(
        args: Vec<String>,
        config: Option<UnifiedConfig>,
        report: &ExternalEnvCompatReport,
    ) -> Result<CommandResult, clap::Error> {
        let cli = Self::parse_cli_prepared(args, config, report)?;
        match cli.command {
            Some(Commands::Info(opts)) => Ok(CommandResult::Info(opts)),
            Some(Commands::Tls(opts)) => Ok(CommandResult::Tls(opts)),
            Some(Commands::Diagnose(opts)) => Ok(CommandResult::Diagnose(opts)),
            Some(Commands::Server(opts)) => Self::server_command_result(Self::from_server_opts(*opts)),
            None => Self::server_command_result(Self::from_server_opts(default_server_opts())),
        }
    }

    /// Parse from preprocessed args. Supports both `rustfs <volume>` and `rustfs server <volume>`.
    #[allow(dead_code)] // used in config_test
    pub fn parse_from<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let (args, report, config) = Self::prepare_args(args).unwrap_or_else(|error| error.exit());
        let cli = Self::parse_cli_prepared(args, config, &report).unwrap_or_else(|error| error.exit());
        match cli.command {
            Some(Commands::Server(opts)) => Self::from_server_opts(*opts),
            Some(Commands::Info(_)) | Some(Commands::Tls(_)) | Some(Commands::Diagnose(_)) => {
                Self::from_server_opts(default_server_opts())
            }
            None => {
                // Default to server with empty volumes (will be filled from env)
                Self::from_server_opts(default_server_opts())
            }
        }
    }

    /// Parse from preprocessed args and return the command type.
    /// Returns Ok(Info(opts)) if info command, Ok(Server(opts)) if server command.
    pub fn parse_command<I, T>(args: I) -> Result<CommandResult, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let (args, report, config) = Self::prepare_args(args)?;
        match Self::parse_command_prepared(args, config, &report) {
            Ok(result) => Ok(result),
            Err(e) => {
                // Handle help and version display - these are not real errors
                if e.kind() == clap::error::ErrorKind::DisplayHelp || e.kind() == clap::error::ErrorKind::DisplayVersion {
                    // Print the help/version message and exit successfully
                    e.print().ok();
                    std::process::exit(0);
                }
                Err(e)
            }
        }
    }

    // Helper to convert Opt to CommandResult::Server with error handling
    fn server_command_result(opt: Opt) -> Result<CommandResult, clap::Error> {
        Ok(Server(Box::new(Config::from_opt(opt).map_err(|e| {
            clap::Error::raw(clap::error::ErrorKind::ValueValidation, e.to_string())
        })?)))
    }

    /// Try parse from args, returns error on invalid input.
    #[allow(dead_code)] // used in config_test
    pub fn try_parse_from<I, T>(args: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let (args, report, config) = Self::prepare_args(args)?;
        let cli = Self::parse_cli_prepared(args, config, &report)?;
        match cli.command {
            Some(Commands::Server(opts)) => Ok(Self::from_server_opts(*opts)),
            Some(Commands::Info(_)) | Some(Commands::Tls(_)) | Some(Commands::Diagnose(_)) => {
                Err(clap::Error::new(clap::error::ErrorKind::DisplayHelp))
            }
            None => {
                // Default to server with empty volumes
                Ok(Self::from_server_opts(default_server_opts()))
            }
        }
    }

    /// Parse from env::args(). Used by Config::parse().
    #[allow(dead_code)] // used in config_test
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        Self::parse_from(args)
    }
}

fn configure_external_env_aliases(command: clap::Command, report: &ExternalEnvCompatReport) -> clap::Command {
    let aliases: BTreeMap<&str, OsString> = report
        .mapped_pairs
        .iter()
        .filter_map(|(source, canonical)| std::env::var_os(source).map(|value| (canonical.as_str(), value)))
        .collect();

    command.mut_subcommand("server", |server| {
        server.mut_args(|argument| {
            let Some(value) = argument
                .get_env()
                .and_then(|key| key.to_str())
                .and_then(|key| aliases.get(key))
            else {
                return argument;
            };
            let argument = argument.default_value_os(value.clone());
            if argument.get_id() == "volumes" {
                argument.required(false)
            } else {
                argument
            }
        })
    })
}
