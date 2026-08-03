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

const DEFAULT_PRODUCT_VERSION: &str = include_str!("../ZFFS_VERSION");

fn product_version(raw: &str) -> Result<&str, std::io::Error> {
    let version = raw.trim();
    let version = version.strip_prefix('v').unwrap_or(version);
    if version.is_empty()
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
    {
        return Err(std::io::Error::other("ZFFS_VERSION must be a non-empty release identifier"));
    }
    Ok(version)
}

fn main() -> shadow_rs::SdResult<()> {
    println!("cargo:rerun-if-changed=../ZFFS_VERSION");
    println!("cargo:rerun-if-env-changed=ZFFS_VERSION");
    let configured_version = std::env::var("ZFFS_VERSION").unwrap_or_else(|_| DEFAULT_PRODUCT_VERSION.to_string());
    println!("cargo:rustc-env=ZFFS_VERSION={}", product_version(&configured_version)?);
    shadow_rs::ShadowBuilder::builder().build()?;
    Ok(())
}
