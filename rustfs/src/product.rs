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

pub(crate) const NAME: &str = "ZfFS";
pub(crate) const FULL_NAME: &str = "ZfFS Object Storage Server";
pub(crate) const BINARY_NAME: &str = "zffs";
pub(crate) const VERSION: &str = env!("ZFFS_VERSION");
pub(crate) const UPSTREAM_NAME: &str = "RustFS";
pub(crate) const UPSTREAM_VERSION: &str = crate::version::build::PKG_VERSION;
