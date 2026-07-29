use std::path::PathBuf;

use nix::unistd::{Gid, Group, Uid, User};

use crate::error::{Error, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserAccount {
    pub name: String,
    pub uid: u32,
    pub gid: u32,
    pub home: PathBuf,
}

pub trait IdentityProvider: Send + Sync {
    fn effective_uid(&self) -> u32;
    fn current_uid(&self) -> u32;

    /// Resolves one local account.
    ///
    /// # Errors
    ///
    /// Returns an error when the account database cannot be queried.
    fn user(&self, name: &str) -> Result<Option<UserAccount>>;

    /// Resolves one local group identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the group database cannot be queried.
    fn group_gid(&self, name: &str) -> Result<Option<u32>>;
}

#[derive(Debug, Default)]
pub struct SystemIdentity;

impl IdentityProvider for SystemIdentity {
    fn effective_uid(&self) -> u32 {
        Uid::effective().as_raw()
    }

    fn current_uid(&self) -> u32 {
        Uid::current().as_raw()
    }

    fn user(&self, name: &str) -> Result<Option<UserAccount>> {
        User::from_name(name)
            .map_err(|error| Error::new("identity_error", format!("cannot resolve user: {error}")))
            .map(|account| {
                account.map(|account| UserAccount {
                    name: account.name,
                    uid: account.uid.as_raw(),
                    gid: account.gid.as_raw(),
                    home: account.dir,
                })
            })
    }

    fn group_gid(&self, name: &str) -> Result<Option<u32>> {
        Group::from_name(name)
            .map_err(|error| Error::new("identity_error", format!("cannot resolve group: {error}")))
            .map(|group| group.map(|group| group.gid.as_raw()))
    }
}

#[must_use]
pub const fn uid(value: u32) -> Uid {
    Uid::from_raw(value)
}

#[must_use]
pub const fn gid(value: u32) -> Gid {
    Gid::from_raw(value)
}
