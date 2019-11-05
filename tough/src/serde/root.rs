// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::serde::decoded::{Decoded, Hex};
use crate::serde::key::Key;
use crate::serde::{Metadata, Role};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::num::NonZeroU64;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "_type")]
#[serde(rename = "root")]
pub(crate) struct Root {
    pub(crate) consistent_snapshot: bool,
    pub(crate) expires: DateTime<Utc>,
    pub(crate) keys: BTreeMap<Decoded<Hex>, Key>,
    pub(crate) roles: BTreeMap<Role, RoleKeys>,
    pub(crate) spec_version: String,
    pub(crate) version: NonZeroU64,
}

impl Root {
    pub(crate) fn keys(&self, role: Role) -> Vec<Key> {
        let keyids = match self.roles.get(&role) {
            Some(role_keys) => &role_keys.keyids,
            None => return Vec::new(),
        };
        keyids
            .iter()
            .filter_map(|keyid| self.keys.get(keyid).cloned())
            .collect()
    }
}

impl Metadata for Root {
    const ROLE: Role = Role::Root;

    fn expires(&self) -> &DateTime<Utc> {
        &self.expires
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct RoleKeys {
    pub(crate) keyids: Vec<Decoded<Hex>>,
    pub(crate) threshold: NonZeroU64,
}
