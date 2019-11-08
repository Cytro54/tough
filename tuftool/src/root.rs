// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::error::{self, Result};
use chrono::Utc;
use maplit::hashmap;
use serde::Serialize;
use snafu::{OptionExt, ResultExt};
use std::collections::HashMap;
use std::io::Write;
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use tempfile::NamedTempFile;
use tough_schema::{RoleKeys, RoleType, Root, Signed};

#[derive(Debug, StructOpt)]
pub(crate) enum Command {
    /// Create a new root.json metadata file
    Init {
        /// Path where root.json is written
        path: PathBuf,
    },
}

macro_rules! role_keys {
    ($threshold:expr) => {
        RoleKeys {
            keyids: Vec::new(),
            threshold: NonZeroU64::new($threshold).unwrap(),
            _extra: HashMap::new(),
        }
    };

    () => {
        // absurdly high threshold value so that someone realizes they need to change this
        role_keys!(1507)
    };
}

impl Command {
    pub(crate) fn run(&self) -> Result<()> {
        match self {
            Command::Init { path } => write_json(
                path,
                &Signed {
                    signed: Root {
                        spec_version: "1.0".to_owned(),
                        consistent_snapshot: true,
                        version: NonZeroU64::new(1).unwrap(),
                        expires: Utc::now(),
                        keys: HashMap::new(),
                        roles: hashmap! {
                            RoleType::Root => role_keys!(),
                            RoleType::Snapshot => role_keys!(),
                            RoleType::Targets => role_keys!(),
                            RoleType::Timestamp => role_keys!(),
                        },
                        _extra: HashMap::new(),
                    },
                    signatures: Vec::new(),
                },
            ),
        }
    }
}

fn write_json<T: Serialize>(path: &Path, json: &T) -> Result<()> {
    // Use `tempfile::NamedTempFile::persist` to perform an atomic file write.
    let parent = path.parent().context(error::PathParent { path })?;
    let mut writer =
        NamedTempFile::new_in(parent).context(error::FileTempCreate { path: parent })?;
    serde_json::to_writer_pretty(&mut writer, json).context(error::FileWriteJson { path })?;
    writer.write_all(b"\n").context(error::FileWrite { path })?;
    writer.persist(path).context(error::FilePersist { path })?;
    Ok(())
}
