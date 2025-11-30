use std::fs::{Metadata as StdMetadata, OpenOptions};

/// クロスプラットフォームなファイルパーミッション表現。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FilePermissions {
    unix_mode: Option<u32>,
    windows_attributes: Option<u32>,
}

impl FilePermissions {
    /// Unix 系 OS のモード値を保持したパーミッションを生成する。
    pub const fn unix_mode(mode: u32) -> Self {
        Self {
            unix_mode: Some(mode),
            windows_attributes: None,
        }
    }

    /// Windows 系 OS の `FILE_ATTRIBUTE_*` を保持したパーミッションを生成する。
    pub const fn windows_attributes(attributes: u32) -> Self {
        Self {
            unix_mode: None,
            windows_attributes: Some(attributes),
        }
    }

    /// Unix OS で取得したモード値を返す。
    pub fn unix_mode_value(&self) -> Option<u32> {
        self.unix_mode
    }

    /// Windows OS で取得した属性値を返す。
    pub fn windows_attributes_value(&self) -> Option<u32> {
        self.windows_attributes
    }

    pub(crate) fn from_metadata(metadata: &StdMetadata) -> Self {
        let mut result = FilePermissions::default();
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::MetadataExt;
            result.unix_mode = Some(metadata.mode());
        }
        #[cfg(target_family = "windows")]
        {
            use std::os::windows::fs::MetadataExt;
            result.windows_attributes = Some(metadata.file_attributes());
        }
        result
    }

    pub(crate) fn apply_to_open_options(&self, opts: &mut OpenOptions) {
        #[cfg(target_family = "unix")]
        {
            if let Some(mode) = self.unix_mode {
                use std::os::unix::fs::OpenOptionsExt;
                opts.mode(mode);
            }
        }
        #[cfg(target_family = "windows")]
        {
            if let Some(attributes) = self.windows_attributes {
                use std::os::windows::fs::OpenOptionsExt;
                opts.attributes(attributes);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.unix_mode.is_none() && self.windows_attributes.is_none()
    }
}
