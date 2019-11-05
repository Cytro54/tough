// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::error::{self, Result};
use serde::Serialize;
use snafu::{ensure, ResultExt};
use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub(crate) struct Datastore(PathBuf);

fn check_permissions<P: AsRef<Path>>(path: P) -> Result<()> {
    let metadata = match fs::metadata(&path) {
        Ok(meta) => meta,
        Err(err) => match err.kind() {
            ErrorKind::NotFound => return Ok(()),
            _ => {
                return Err(err).context(error::DatastoreMetadata {
                    path: path.as_ref(),
                })
            }
        },
    };
    ensure!(
        metadata.permissions().mode() & 0o022 == 0,
        error::DatastorePermissions {
            path: path.as_ref(),
            mode: metadata.permissions().mode()
        }
    );
    Ok(())
}

impl Datastore {
    pub(crate) fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        check_permissions(&path)?;
        Ok(Self(path.as_ref().to_owned()))
    }

    pub(crate) fn reader(&self, file: &str) -> Result<Option<impl Read>> {
        let path = self.0.join(file);
        check_permissions(&path)?;
        match File::open(&path) {
            Ok(file) => Ok(Some(file)),
            Err(err) => match err.kind() {
                ErrorKind::NotFound => Ok(None),
                _ => Err(err).context(error::DatastoreOpen { path: &path }),
            },
        }
    }

    pub(crate) fn create<T: Serialize>(&self, file: &str, value: &T) -> Result<()> {
        let path = self.0.join(file);
        check_permissions(&path)?;
        let mut f = File::create(&path).context(error::DatastoreCreate { path: &path })?;
        f.metadata()
            .context(error::DatastoreMetadata { path: &path })?
            .permissions()
            .set_mode(0o644);
        serde_json::to_writer_pretty(&mut f, value).context(error::JsonSerialization {
            what: format!("{} in datastore", file),
        })?;
        Ok(())
    }

    pub(crate) fn remove(&self, file: &str) -> Result<()> {
        let path = self.0.join(file);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) => match err.kind() {
                ErrorKind::NotFound => Ok(()),
                _ => Err(err).context(error::DatastoreRemove { path: &path }),
            },
        }
    }
}
