use glob::{glob_with, GlobError, MatchOptions, PatternError};

use crate::io::{record_io_operation, take_io_effects_snapshot, FsAdapter, IoError};
use crate::text::Str;

use super::{validate_input, PathBuf, PathError, PathErrorKind, PathResult};

const MATCH_OPTIONS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: false,
    require_literal_leading_dot: false,
};
/// `glob(pattern)` を実装する。
pub fn glob(pattern: Str<'_>) -> PathResult<Vec<PathBuf>> {
    let pattern_str = pattern.as_str();
    validate_input(pattern_str)?;
    let pattern_owned = pattern_str.to_owned();

    record_io_operation(0);
    FsAdapter::global()
        .ensure_read_capability()
        .map_err(|err| {
            let effects = take_io_effects_snapshot();
            path_error_from_io(&pattern_owned, err).with_effects(effects)
        })?;

    record_io_operation(0);
    let paths = glob_with(&pattern_owned, MATCH_OPTIONS).map_err(|err| {
        let effects = take_io_effects_snapshot();
        path_error_from_pattern(&pattern_owned, err).with_effects(effects)
    })?;

    let mut matches: Vec<PathBuf> = Vec::new();
    for entry in paths {
        let path = entry.map_err(|err| {
            let effects = take_io_effects_snapshot();
            path_error_from_glob(&pattern_owned, err).with_effects(effects)
        })?;
        matches.push(PathBuf::from_std(path));
    }
    matches.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    let _ = take_io_effects_snapshot();
    Ok(matches)
}

fn path_error_from_pattern(pattern: &str, err: PatternError) -> PathError {
    PathError::new(
        PathErrorKind::InvalidPattern,
        format!("invalid glob pattern `{pattern}`: {err}"),
    )
    .with_input(pattern.to_owned())
    .with_glob_pattern(pattern.to_owned())
}

fn path_error_from_glob(pattern: &str, err: GlobError) -> PathError {
    let offending_path = err.path().to_string_lossy().into_owned();
    PathError::new(
        PathErrorKind::Io,
        format!(
            "failed to enumerate `{pattern}` at `{offending_path}`: {}",
            err.error()
        ),
    )
    .with_input(pattern.to_owned())
    .with_glob_pattern(pattern.to_owned())
    .with_glob_offending_path(offending_path)
}

fn path_error_from_io(pattern: &str, err: IoError) -> PathError {
    PathError::new(
        PathErrorKind::Io,
        format!("glob requires io.fs.read capability: {err}"),
    )
    .with_input(pattern.to_owned())
    .with_glob_pattern(pattern.to_owned())
}
