// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::error::{self, Result};
use crate::key::KeyPair;
use crate::source::KeySource;
use crate::{load_file, write_file};
use chrono::{DateTime, Timelike, Utc};
use maplit::hashmap;
use snafu::{ensure, ResultExt};
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::path::PathBuf;
use structopt::StructOpt;
use tough_schema::key::Key;
use tough_schema::{RoleKeys, RoleType, Root, Signed};

#[derive(Debug, StructOpt)]
pub(crate) enum Command {
    /// Create a new root.json metadata file
    Init {
        /// Path to new root.json
        path: PathBuf,
    },
    /// Set the expiration time for root.json
    Expire {
        /// Path to root.json
        path: PathBuf,
        /// When to expire
        time: DateTime<Utc>,
    },
    /// Set the signature count threshold for a role
    SetThreshold {
        /// Path to root.json
        path: PathBuf,
        /// The role to set
        role: RoleType,
        /// The new threshold
        threshold: NonZeroU64,
    },
    /// Add a key (public or private) to a role
    AddKey {
        /// Path to root.json
        path: PathBuf,
        /// The role to add the key to
        role: RoleType,
        /// The new key
        key_path: KeySource,
    },
    /// Generate a new RSA key pair, saving it to a file, and add it to a role
    GenRsaKey {
        /// Path to root.json
        path: PathBuf,
        /// The role to add the key to
        role: RoleType,
        /// Where to write the new key
        key_path: KeySource,
        /// Bit length of new key
        #[structopt(short = "b", long = "bits", default_value = "2048")]
        bits: u16,
        /// Public exponent of new key
        #[structopt(short = "e", long = "exp", default_value = "65537")]
        exponent: u32,
    },
}

macro_rules! role_keys {
    ($threshold:expr) => {
        RoleKeys {
            keyids: Vec::new(),
            threshold: $threshold,
            _extra: HashMap::new(),
        }
    };

    () => {
        // absurdly high threshold value so that someone realizes they need to change this
        role_keys!(NonZeroU64::new(1507).unwrap())
    };
}

impl Command {
    pub(crate) fn run(&self) -> Result<()> {
        match self {
            Command::Init { path } => write_file(
                path,
                &Signed {
                    signed: Root {
                        spec_version: "1.0".to_owned(),
                        consistent_snapshot: true,
                        version: NonZeroU64::new(1).unwrap(),
                        expires: round_time(Utc::now()),
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
            Command::Expire { path, time } => {
                let mut root: Signed<Root> = load_file(path)?;
                root.signed.expires = round_time(*time);
                write_file(path, &root)
            }
            Command::SetThreshold {
                path,
                role,
                threshold,
            } => {
                let mut root: Signed<Root> = load_file(path)?;
                root.signed
                    .roles
                    .entry(*role)
                    .and_modify(|rk| rk.threshold = *threshold)
                    .or_insert_with(|| role_keys!(*threshold));
                write_file(path, &root)
            }
            Command::AddKey {
                path,
                role,
                key_path,
            } => {
                let mut root: Signed<Root> = load_file(path)?;
                let key_pair = key_path.as_public_key()?;
                add_key(&mut root.signed, *role, key_pair)?;
                write_file(path, &root)
            }
            Command::GenRsaKey {
                path,
                role,
                key_path,
                bits,
                exponent,
            } => {
                let mut root: Signed<Root> = load_file(path)?;

                // ring doesn't support RSA key generation yet
                // https://github.com/briansmith/ring/issues/219
                let mut command = std::process::Command::new("openssl");
                command.args(&["genpkey", "-algorithm", "RSA", "-pkeyopt"]);
                command.arg(format!("rsa_keygen_bits:{}", bits));
                command.arg("-pkeyopt");
                command.arg(format!("rsa_keygen_pubexp:{}", exponent));

                let command_str = format!("{:?}", command);
                let output = command.output().context(error::CommandExec {
                    command_str: &command_str,
                })?;
                ensure!(
                    output.status.success(),
                    error::CommandStatus {
                        command_str: &command_str,
                        status: output.status
                    }
                );
                let stdout =
                    String::from_utf8(output.stdout).context(error::CommandUtf8 { command_str })?;

                let key_pair = KeyPair::parse(stdout.as_bytes())?;
                add_key(&mut root.signed, *role, key_pair.public_key())?;
                key_path.write(&stdout)?;
                write_file(path, &root)
            }
        }
    }
}

fn round_time(time: DateTime<Utc>) -> DateTime<Utc> {
    // `Timelike::with_nanosecond` returns None only when passed a value >= 2_000_000_000
    time.with_nanosecond(0).unwrap()
}

/// Adds a key to the root role if not already present, and adds its key ID to the specified role.
fn add_key(root: &mut Root, role: RoleType, key: Key) -> Result<()> {
    let key_id = if let Some((key_id, _)) = root
        .keys
        .iter()
        .find(|(_, candidate_key)| key.eq(candidate_key))
    {
        key_id.clone()
    } else {
        // Key isn't present yet, so we need to add it
        let key_id = key.key_id().context(error::KeyId)?;
        ensure!(
            !root.keys.contains_key(&key_id),
            error::KeyDuplicate {
                key_id: hex::encode(&key_id)
            }
        );
        root.keys.insert(key_id.clone(), key);
        key_id
    };

    let entry = root.roles.entry(role).or_insert_with(|| role_keys!());
    if !entry.keyids.contains(&key_id) {
        entry.keyids.push(key_id);
    }

    Ok(())
}
