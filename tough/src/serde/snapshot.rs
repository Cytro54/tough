// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::serde::{Meta, Metadata, Role};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::num::NonZeroU64;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "_type")]
#[serde(rename = "snapshot")]
pub(crate) struct Snapshot {
    pub(crate) expires: DateTime<Utc>,
    pub(crate) meta: BTreeMap<String, Meta>,
    pub(crate) spec_version: String,
    pub(crate) version: NonZeroU64,
}

impl Metadata for Snapshot {
    const ROLE: Role = Role::Snapshot;

    fn expires(&self) -> &DateTime<Utc> {
        &self.expires
    }
}
