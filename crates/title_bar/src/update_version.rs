use std::sync::Arc;

use anyhow::anyhow;

use gpui::{Empty, Render};
use semver::Version;
use ui::{Tooltip, UpdateButton, prelude::*};






#[cfg(test)]
mod tests {
    use semver::Version;

    use super::*;

    #[test]
    fn test_version_tooltip_message() {
        let message = UpdateVersion::version_tooltip_message(&Version::new(1, 0, 0));

        assert_eq!(message, "Update to Version: 1.0.0");

        let message = UpdateVersion::version_tooltip_message(
            &"1.0.0+nightly.14d9a4189f058d8736339b06ff2340101eaea5af"
                .parse()
                .unwrap(),
        );

        assert_eq!(
            message,
            "Update to Version: 1.0.0+nightly.14d9a4189f058d8736339b06ff2340101eaea5af"
        );
    }

    #[test]
    fn test_downloading_tooltip_message() {
        let version = Version::new(1, 0, 0);

        let message = UpdateButton::downloading_tooltip_message(&version, None);
        assert_eq!(message, "Update to Version: 1.0.0");

        let message = UpdateButton::downloading_tooltip_message(&version, Some(0.454));
        assert_eq!(message, "Update to Version: 1.0.0 (45% downloaded)");

        let message = UpdateButton::downloading_tooltip_message(&version, Some(1.5));
        assert_eq!(message, "Update to Version: 1.0.0 (100% downloaded)");
    }
}
