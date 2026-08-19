//! Per-authority connection budget for the native download engine.

use crate::util::fetch::{DownloadRoute, ProxyPolicy};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

const MAX_CONNECTIONS_PER_AUTHORITY: usize = 16;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct AuthorityKey {
	authority: String,
	proxy: ProxyPolicy,
}

static AUTHORITY_BUDGETS: LazyLock<
	Mutex<HashMap<AuthorityKey, Arc<Semaphore>>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn budget(route: &DownloadRoute) -> Option<Arc<Semaphore>> {
	let authority = crate::util::fetch::url_authority(&route.url)?;
	let key = AuthorityKey {
		authority,
		proxy: route.proxy,
	};
	let mut budgets = AUTHORITY_BUDGETS.lock();
	if budgets.len() >= 256 {
		budgets.retain(|_, budget| Arc::strong_count(budget) > 1);
	}
	Some(
		budgets
			.entry(key)
			.or_insert_with(|| {
				Arc::new(Semaphore::new(MAX_CONNECTIONS_PER_AUTHORITY))
			})
			.clone(),
	)
}

pub(crate) async fn acquire(
	route: &DownloadRoute,
) -> Result<Option<OwnedSemaphorePermit>, tokio::sync::AcquireError> {
	match budget(route) {
		Some(budget) => budget.acquire_owned().await.map(Some),
		None => Ok(None),
	}
}

pub(crate) fn try_acquire(
	route: &DownloadRoute,
) -> Result<Option<OwnedSemaphorePermit>, TryAcquireError> {
	match budget(route) {
		Some(budget) => budget.try_acquire_owned().map(Some),
		None => Ok(None),
	}
}

pub(crate) fn available(route: &DownloadRoute) -> usize {
	budget(route)
		.map(|budget| budget.available_permits())
		.unwrap_or(MAX_CONNECTIONS_PER_AUTHORITY)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::util::fetch::DownloadRouteSource;

	fn route() -> DownloadRoute {
		DownloadRoute {
			url: "https://budget.example/file".to_string(),
			source: DownloadRouteSource::Official,
			is_mirror: false,
			allow_sensitive_headers: true,
			supports_range: true,
			proxy: ProxyPolicy::Direct,
		}
	}

	#[tokio::test]
	async fn authority_budget_is_bounded() {
		let route = route();
		let mut permits = Vec::new();
		for _ in 0..MAX_CONNECTIONS_PER_AUTHORITY {
			permits.push(acquire(&route).await.unwrap());
		}
		assert!(matches!(try_acquire(&route), Err(TryAcquireError::NoPermits)));
		drop(permits);
		assert!(try_acquire(&route).is_ok());
	}
}
