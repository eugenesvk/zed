use std::{fmt, sync::Arc};

use async_lock::{RwLock, RwLockUpgradableReadGuard, RwLockWriteGuard};
use cloud_api_types::OrganizationId;

use crate::{ClientApiError, CloudApiClient};




struct CachedLlmApiToken {
    /// The organization ID the token was minted for.
    organization_id: OrganizationId,
    token: String,
}

impl fmt::Debug for CachedLlmApiToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedLlmApiToken")
            .field("organization_id", &self.organization_id)
            .field("token", &"<redacted>")
            .finish()
    }
}


