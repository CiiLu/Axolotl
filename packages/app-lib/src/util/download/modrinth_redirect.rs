//! Modrinth CDN redirect normalization for native download requests.

use url::Url;

use crate::util::fetch::{
    MODRINTH_CDN_LEGACY_HOST, MODRINTH_CDN_OFFICIAL_HOST, TIANPAO_HOST,
};

pub(crate) fn canonical_cdn_url(url: &str) -> String {
    let Ok(mut parsed) = Url::parse(url) else {
        return url.to_string();
    };
    if parsed.scheme() == "https"
        && parsed.host_str().is_some_and(|host| {
            host.eq_ignore_ascii_case(MODRINTH_CDN_LEGACY_HOST)
        })
    {
        let _ = parsed.set_host(Some(MODRINTH_CDN_OFFICIAL_HOST));
    }
    parsed.into()
}

pub(crate) fn repair_official_redirect(
    original: &Url,
    redirect: &Url,
    location: &str,
) -> Option<Url> {
    if location.is_ascii()
        || !redirect.host_str().is_some_and(|host| {
            host.eq_ignore_ascii_case(MODRINTH_CDN_LEGACY_HOST)
                || host.eq_ignore_ascii_case(MODRINTH_CDN_OFFICIAL_HOST)
        })
        || original.path().is_empty()
    {
        return None;
    }
    let mut repaired = redirect.clone();
    repaired.set_path(original.path());
    repaired.set_query(original.query());
    repaired.set_fragment(original.fragment());
    Some(repaired)
}

pub(crate) fn is_official_redirect(location: Option<&str>) -> bool {
    let Some(location) = location.filter(|location| {
        location.len() <= 8 * 1024
            && location.is_ascii()
            && location
                .get(..8)
                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
    }) else {
        return false;
    };
    let authority = location[8..]
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    authority.eq_ignore_ascii_case(MODRINTH_CDN_OFFICIAL_HOST)
        || authority
            .eq_ignore_ascii_case(&format!("{MODRINTH_CDN_OFFICIAL_HOST}:443"))
        || authority.eq_ignore_ascii_case(MODRINTH_CDN_LEGACY_HOST)
        || authority
            .eq_ignore_ascii_case(&format!("{MODRINTH_CDN_LEGACY_HOST}:443"))
}

/// Converts Tianpao's Modrinth redirect to the legacy CDN host so the
/// configured DNS override for `cdn.modrinth.com` is retained.
pub(crate) fn tianpao_redirect_target(
    current: &Url,
    redirect: &Url,
) -> Option<Url> {
    if current.host_str() != Some(TIANPAO_HOST)
        || !current.path().starts_with("/data/")
        || !redirect.host_str().is_some_and(|host| {
            host.eq_ignore_ascii_case(MODRINTH_CDN_LEGACY_HOST)
                || host.eq_ignore_ascii_case(MODRINTH_CDN_OFFICIAL_HOST)
        })
    {
        return None;
    }
    let mut target = redirect.clone();
    target.set_host(Some(MODRINTH_CDN_LEGACY_HOST)).ok()?;
    Some(target)
}
