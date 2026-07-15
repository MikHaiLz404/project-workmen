//! Profile lifecycle (Draft -> Active -> Locked).

use std::collections::BTreeMap;

use thiserror::Error;

use crate::model::Profile;
use crate::model::ProfileId;
use crate::model::ProfileState;

/// Errors from the lifecycle module.
#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("profile {0:?} not found")]
    NotFound(ProfileId),
    #[error("cannot edit profile {id:?}: locked")]
    Locked { id: ProfileId },
    #[error("cannot lock profile {id:?}: not Active (current: {current:?})")]
    NotActive {
        id: ProfileId,
        current: ProfileState,
    },
    #[error("unlock reason must not be empty")]
    EmptyUnlockReason,
}

/// Lifecycle state for a profile. The lifecycle tracks the
/// per-profile unlock reason (which `Profile` does not carry in
/// the model) separately so the model stays clean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleEntry {
    pub unlock_reason: Option<String>,
}

/// In-memory CRUD over a project's profile set. The lifecycle
/// preserves insertion order so iteration is stable.
#[derive(Clone, Debug, Default)]
pub struct ProfileLifecycle {
    profiles: BTreeMap<ProfileId, Profile>,
    lifecycle: BTreeMap<ProfileId, LifecycleEntry>,
}

impl ProfileLifecycle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a profile.
    pub fn upsert(&mut self, profile: Profile) {
        let id = profile.id.clone();
        self.profiles.insert(id.clone(), profile);
        self.lifecycle.entry(id).or_insert(LifecycleEntry {
            unlock_reason: None,
        });
    }

    /// Get a profile by id.
    pub fn get(&self, id: &ProfileId) -> Option<&Profile> {
        self.profiles.get(id)
    }

    /// Get the lifecycle entry (unlock reason) for a profile.
    pub fn lifecycle(&self, id: &ProfileId) -> Option<&LifecycleEntry> {
        self.lifecycle.get(id)
    }

    /// Iterate the profiles in id-sorted order.
    pub fn iter(&self) -> impl Iterator<Item = (&ProfileId, &Profile)> {
        self.profiles.iter()
    }

    /// Edit a profile in place. The lifecycle enforces state
    /// transitions: Locked profiles cannot be edited.
    pub fn edit<F>(&mut self, id: &ProfileId, f: F) -> Result<(), LifecycleError>
    where
        F: FnOnce(&mut Profile),
    {
        let profile = self
            .profiles
            .get(id)
            .ok_or(LifecycleError::NotFound(id.clone()))?;
        if matches!(profile.state, ProfileState::Locked) {
            return Err(LifecycleError::Locked { id: id.clone() });
        }
        let mut profile = profile.clone();
        f(&mut profile);
        profile.profile_revision = profile.profile_revision.saturating_add(1);
        self.profiles.insert(id.clone(), profile);
        Ok(())
    }

    /// Lock a profile. The profile must currently be `Active`.
    /// The `reason` is recorded so the next `unlock` call can echo
    /// it back.
    pub fn lock(&mut self, id: &ProfileId, reason: String) -> Result<(), LifecycleError> {
        let profile = self
            .profiles
            .get(id)
            .ok_or(LifecycleError::NotFound(id.clone()))?;
        if !matches!(profile.state, ProfileState::Active) {
            return Err(LifecycleError::NotActive {
                id: id.clone(),
                current: profile.state,
            });
        }
        let mut profile = profile.clone();
        profile.state = ProfileState::Locked;
        profile.profile_revision = profile.profile_revision.saturating_add(1);
        self.profiles.insert(id.clone(), profile);
        self.lifecycle.insert(
            id.clone(),
            LifecycleEntry {
                unlock_reason: Some(reason),
            },
        );
        Ok(())
    }

    /// Unlock a profile. The `reason` must be non-empty.
    pub fn unlock(&mut self, id: &ProfileId, reason: String) -> Result<(), LifecycleError> {
        if reason.trim().is_empty() {
            return Err(LifecycleError::EmptyUnlockReason);
        }
        let profile = self
            .profiles
            .get(id)
            .ok_or(LifecycleError::NotFound(id.clone()))?;
        if !matches!(profile.state, ProfileState::Locked) {
            return Err(LifecycleError::NotActive {
                id: id.clone(),
                current: profile.state,
            });
        }
        let mut profile = profile.clone();
        profile.state = ProfileState::Active;
        profile.profile_revision = profile.profile_revision.saturating_add(1);
        self.profiles.insert(id.clone(), profile);
        self.lifecycle.insert(
            id.clone(),
            LifecycleEntry {
                unlock_reason: None,
            },
        );
        Ok(())
    }
}
