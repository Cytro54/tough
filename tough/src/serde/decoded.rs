// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::error::{self, Compat, Error};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use snafu::ResultExt;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::ops::Deref;

/// Represents bytes decoded from a string.
///
/// The type parameter `T` represents what kind of data the original string stores (e.g.
/// hex-encoded bytes, or a PEM-encoded key).
///
/// The original string is stored so that it can be re-`Serialize`d for the purposes of verifying
/// signatures.
pub(crate) struct Decoded<T: Decode> {
    bytes: Vec<u8>,
    original: String,
    spooky: PhantomData<T>,
}

impl<T: Decode> Decoded<T> {
    /// Consume this object and return its bytes.
    pub(crate) fn into_vec(self) -> Vec<u8> {
        self.bytes
    }
}

// =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=

/// A type that represents how data can be converted from a string to bytes.
///
/// Generally structs that implement `Decode` will be unit-like structs that just implement the one
/// required method.
pub(crate) trait Decode {
    /// Convert a string to bytes.
    ///
    /// The "error" string returned from this method will immediately be wrapped into a
    /// [`serde::de::Error`].
    fn parse(s: &str) -> Result<Vec<u8>, Error>;
}

/// [`Decode`] implementation for hex-encoded strings.
pub(crate) struct Hex;

impl Decode for Hex {
    fn parse(s: &str) -> Result<Vec<u8>, Error> {
        hex::decode(s).context(error::HexDecode)
    }
}

/// [`Decode`] implementation for PEM-encoded keys.
pub(crate) struct Pem;

impl Decode for Pem {
    fn parse(s: &str) -> Result<Vec<u8>, Error> {
        pem::parse(s)
            .map(|pem| pem.contents)
            .map_err(Compat)
            .context(error::PemDecode)
    }
}

// =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=

impl<'de, T: Decode> Deserialize<'de> for Decoded<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let original = String::deserialize(deserializer)?;
        Ok(Self {
            bytes: T::parse(&original).map_err(D::Error::custom)?,
            original,
            spooky: PhantomData,
        })
    }
}

impl<T: Decode> Serialize for Decoded<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.original)
    }
}

// =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=

impl<T: Decode> Deref for Decoded<T> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.bytes
    }
}

impl<T: Decode> Debug for Decoded<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.original, f)
    }
}

impl<T: Decode> Clone for Decoded<T> {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
            original: self.original.clone(),
            spooky: PhantomData,
        }
    }
}

impl<T: Decode> PartialEq for Decoded<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes.eq(&other.bytes)
    }
}

impl<T: Decode> Eq for Decoded<T> {}

impl<T: Decode> PartialOrd for Decoded<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.bytes.partial_cmp(&other.bytes)
    }
}

impl<T: Decode> Ord for Decoded<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes.cmp(&other.bytes)
    }
}
