// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: MIT OR Apache-2.0

mod decoded;
mod key;
mod root;
mod snapshot;
mod targets;
mod timestamp;

pub(crate) use root::Root;
pub(crate) use snapshot::Snapshot;
pub(crate) use targets::{Target, Targets};
pub(crate) use timestamp::Timestamp;

use crate::error::{self, Result};
use crate::serde::decoded::{Decoded, Hex};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_plain::forward_display_to_serde;
use snafu::{ensure, OptionExt, ResultExt};
use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Root,
    Snapshot,
    Targets,
    Timestamp,
}

forward_display_to_serde!(Role);

pub(crate) trait Metadata {
    const ROLE: Role;

    fn expires(&self) -> &DateTime<Utc>;
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Signed<T> {
    pub(crate) signatures: Vec<Signature>,
    pub(crate) signed: T,
}

#[allow(clippy::use_self)] // false positive
impl<T: Metadata + Serialize> Signed<T> {
    pub(crate) fn verify(&self, root: &Signed<Root>) -> Result<()> {
        let role_keys = root
            .signed
            .roles
            .get(&T::ROLE)
            .context(error::MissingRole { role: T::ROLE })?;
        let mut valid = 0;

        // TODO(iliana): actually implement Canonical JSON instead of just hoping that what we get
        // out of serde_json is Canonical JSON
        let data = serde_json::to_vec(&self.signed).context(error::JsonSerialization)?;

        for signature in &self.signatures {
            if role_keys.keyids.contains(&signature.keyid) {
                if let Some(key) = root.signed.keys.get(&signature.keyid) {
                    if key.verify(&data, &signature.sig) {
                        valid += 1;
                    }
                }
            }
        }

        ensure!(
            valid >= u64::from(role_keys.threshold),
            error::SignatureThreshold {
                role: T::ROLE,
                threshold: role_keys.threshold,
                valid,
            }
        );
        Ok(())
    }

    pub(crate) fn check_expired(&self) -> Result<()> {
        ensure!(
            Utc::now() < *self.signed.expires(),
            error::ExpiredMetadata { role: T::ROLE }
        );
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Signature {
    pub(crate) keyid: Decoded<Hex>,
    pub(crate) sig: Decoded<Hex>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Meta {
    pub(crate) hashes: Hashes,
    pub(crate) length: usize,
    pub(crate) version: NonZeroU64,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Hashes {
    pub(crate) sha256: Decoded<Hex>,
}

#[cfg(test)]
mod tests {
    use crate::serde::{root::Root, Signed};

    #[test]
    fn simple_rsa() {
        let root: Signed<Root> =
            serde_json::from_str(include_str!("../../tests/data/simple-rsa/root.json")).unwrap();
        root.verify(&root).unwrap();
    }

    #[test]
    fn duplicate_keyid() {
        assert!(serde_json::from_str::<Signed<Root>>(include_str!(
            "../../tests/data/duplicate-keyid/root.json"
        ))
        .is_err());
    }

    #[test]
    fn no_root_json_signatures_is_err() {
        let root: Signed<Root> = serde_json::from_str(include_str!(
            "../../tests/data/no-root-json-signatures/root.json"
        ))
        .expect("should be parsable root.json");
        root.verify(&root)
            .expect_err("missing signature should not verify");
    }

    #[test]
    fn invalid_root_json_signatures_is_err() {
        let root: Signed<Root> = serde_json::from_str(include_str!(
            "../../tests/data/invalid-root-json-signature/root.json"
        ))
        .expect("should be parsable root.json");
        root.verify(&root)
            .expect_err("invalid (unauthentic) root signature should not verify");
    }

    #[test]
    fn expired_root_json_signature_is_err() {
        let root: Signed<Root> = serde_json::from_str(include_str!(
            "../../tests/data/expired-root-json-signature/root.json"
        ))
        .expect("should be parsable root.json");
        root.verify(&root)
            .expect_err("expired root signature should not verify");
    }

    #[test]
    fn mismatched_root_json_keyids_is_err() {
        let root: Signed<Root> = serde_json::from_str(include_str!(
            "../../tests/data/mismatched-root-json-keyids/root.json"
        ))
        .expect("should be parsable root.json");
        root.verify(&root)
            .expect_err("mismatched root role keyids (provided and signed) should not verify");
    }
}
