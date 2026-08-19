//! Policy boundary for the native download engine.

use crate::util::fetch::{DownloadRoute, ProxyPolicy};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeH2IneligibleReason {
	Http1Fallback,
	SystemProxy,
}

impl NativeH2IneligibleReason {
	pub(crate) const fn as_str(self) -> &'static str {
		match self {
			Self::Http1Fallback => "authority is temporarily using HTTP/1.1",
			Self::SystemProxy => "system proxy requires the reqwest transport",
		}
	}
}

pub(crate) fn h2_ineligible_reason(
	route: &DownloadRoute,
) -> Option<NativeH2IneligibleReason> {
	let authority = crate::util::fetch::url_authority(&route.url)?;
	if crate::util::fetch::authority_uses_http1_fallback(&authority) {
		return Some(NativeH2IneligibleReason::Http1Fallback);
	}
	if route.proxy == ProxyPolicy::System && system_proxy_configured() {
		return Some(NativeH2IneligibleReason::SystemProxy);
	}
	None
}

fn system_proxy_configured() -> bool {
	let environment_proxy = [
		"HTTPS_PROXY",
		"https_proxy",
		"HTTP_PROXY",
		"http_proxy",
		"ALL_PROXY",
		"all_proxy",
	]
	.into_iter()
	.any(|name| {
		std::env::var_os(name).is_some_and(|value| !value.is_empty())
	});
	environment_proxy || platform_proxy_configured()
}

#[cfg(windows)]
fn platform_proxy_configured() -> bool {
	use winreg::RegKey;
	use winreg::enums::HKEY_CURRENT_USER;

	let Ok(settings) = RegKey::predef(HKEY_CURRENT_USER)
		.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
	else {
		return false;
	};
	let enabled = settings
		.get_value::<u32, _>("ProxyEnable")
		.is_ok_and(|value| value != 0);
	let auto_configured = settings
		.get_value::<String, _>("AutoConfigURL")
		.is_ok_and(|value| !value.trim().is_empty());
	enabled || auto_configured
}

#[cfg(not(windows))]
fn platform_proxy_configured() -> bool {
	false
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::util::fetch::{
		DownloadRouteSource, ProxyPolicy,
	};

	fn route(proxy: ProxyPolicy) -> DownloadRoute {
		DownloadRoute {
			url: "https://native-policy.example/file".to_string(),
			source: DownloadRouteSource::Official,
			is_mirror: false,
			allow_sensitive_headers: true,
			supports_range: true,
			proxy,
		}
	}

	#[test]
	fn direct_routes_ignore_system_proxy_environment() {
		assert_ne!(
			h2_ineligible_reason(&route(ProxyPolicy::Direct)),
			Some(NativeH2IneligibleReason::SystemProxy)
		);
	}
}
