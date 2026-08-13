use std::path::Path;

pub(crate) fn is_incomplete_browser_download(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    lower.ends_with(".crdownload")
        || lower.ends_with(".part")
        || lower.ends_with(".partial")
        || lower.ends_with(".tmp")
        || lower.ends_with(".download")
}

pub(crate) fn browser_download_file_name_matches(
    actual: &str,
    expected: &str,
) -> bool {
    if is_incomplete_browser_download(actual) {
        return false;
    }
    if actual.eq_ignore_ascii_case(expected) {
        return true;
    }
    let actual = Path::new(actual);
    let expected = Path::new(expected);
    if actual
        .extension()
        .and_then(|extension| extension.to_str())
        .zip(
            expected
                .extension()
                .and_then(|extension| extension.to_str()),
        )
        .is_none_or(|(actual, expected)| !actual.eq_ignore_ascii_case(expected))
    {
        return false;
    }
    let Some(actual_stem) = actual.file_stem().and_then(|stem| stem.to_str())
    else {
        return false;
    };
    let Some(expected_stem) =
        expected.file_stem().and_then(|stem| stem.to_str())
    else {
        return false;
    };
    let actual_stem = actual_stem.to_lowercase();
    let expected_stem = expected_stem.to_lowercase();
    let Some(suffix) = actual_stem.strip_prefix(&expected_stem) else {
        return false;
    };
    let suffix = suffix.trim();
    suffix
        .strip_prefix('(')
        .and_then(|suffix| suffix.strip_suffix(')'))
        .is_some_and(|number| {
            !number.is_empty()
                && number.bytes().all(|byte| byte.is_ascii_digit())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_duplicates_match_but_incomplete_files_do_not() {
        for actual in [
            "example-mod.jar",
            "EXAMPLE-MOD.JAR",
            "example-mod (1).jar",
            "example-mod(2).jar",
            "example-mod (123).JAR",
        ] {
            assert!(
                browser_download_file_name_matches(actual, "example-mod.jar"),
                "expected {actual:?} to match"
            );
        }
        for actual in [
            "example-mod-fabric.jar",
            "example-mod (copy).jar",
            "example-mod ().jar",
            "example-mod (1) copy.jar",
            "example-mod (1).zip",
            "other-mod (1).jar",
        ] {
            assert!(
                !browser_download_file_name_matches(actual, "example-mod.jar"),
                "expected {actual:?} not to match"
            );
        }
        for suffix in ["crdownload", "part", "partial", "tmp", "download"] {
            assert!(!browser_download_file_name_matches(
                &format!("example-mod.jar.{suffix}"),
                "example-mod.jar"
            ));
            assert!(!browser_download_file_name_matches(
                &format!("example-mod.jar.{}", suffix.to_ascii_uppercase()),
                "example-mod.jar"
            ));
        }
    }
}
