//! Compatibility mode resolution.
//!
//! Precedence ladder (FR-012):
//! 1. Explicit `--strict` / `--no-strict` flag wins over everything.
//! 2. `RUSTY_PEE_STRICT=1` env var (any truthy value).
//! 3. `argv[0]` basename equals `pee` (after `.exe` strip on Windows).
//! 4. Default mode.

use crate::CompatibilityMode;
use std::ffi::OsStr;
use std::path::Path;

/// Resolve the compatibility mode from CLI flag, env var, and argv[0].
pub fn resolve(
    strict_flag: Option<bool>,
    env_strict: Option<&OsStr>,
    argv0: Option<&OsStr>,
) -> CompatibilityMode {
    if let Some(flag) = strict_flag {
        return if flag {
            CompatibilityMode::Strict
        } else {
            CompatibilityMode::Default
        };
    }
    if let Some(value) = env_strict {
        if env_var_is_truthy(value) {
            return CompatibilityMode::Strict;
        }
    }
    if let Some(arg0) = argv0 {
        if argv0_implies_strict(arg0) {
            return CompatibilityMode::Strict;
        }
    }
    CompatibilityMode::Default
}

fn env_var_is_truthy(value: &OsStr) -> bool {
    let Some(s) = value.to_str() else {
        return false;
    };
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn argv0_implies_strict(arg0: &OsStr) -> bool {
    let Some(stem) = Path::new(arg0).file_stem() else {
        return false;
    };
    stem == OsStr::new("pee")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_strict_flag_wins() {
        assert_eq!(resolve(Some(true), None, None), CompatibilityMode::Strict);
        assert_eq!(
            resolve(Some(false), Some(OsStr::new("1")), Some(OsStr::new("pee"))),
            CompatibilityMode::Default,
            "explicit --no-strict beats env and argv[0]"
        );
    }

    #[test]
    fn env_var_truthy_implies_strict() {
        for v in ["1", "true", "yes", "on", "TRUE", " 1 ", "On"] {
            assert_eq!(
                resolve(None, Some(OsStr::new(v)), None),
                CompatibilityMode::Strict,
                "env value {v:?} should imply strict"
            );
        }
    }

    #[test]
    fn env_var_falsy_does_not_imply_strict() {
        for v in ["0", "false", "no", "off", ""] {
            assert_eq!(
                resolve(None, Some(OsStr::new(v)), None),
                CompatibilityMode::Default,
                "env value {v:?} should NOT imply strict"
            );
        }
    }

    #[test]
    fn argv0_pee_implies_strict() {
        assert_eq!(
            resolve(None, None, Some(OsStr::new("pee"))),
            CompatibilityMode::Strict
        );
        assert_eq!(
            resolve(None, None, Some(OsStr::new("/usr/local/bin/pee"))),
            CompatibilityMode::Strict
        );
        assert_eq!(
            resolve(None, None, Some(OsStr::new("pee.exe"))),
            CompatibilityMode::Strict,
            "argv[0] = pee.exe must imply strict (file_stem strips .exe)"
        );
    }

    #[test]
    fn argv0_rusty_pee_does_not_imply_strict() {
        assert_eq!(
            resolve(None, None, Some(OsStr::new("rusty-pee"))),
            CompatibilityMode::Default
        );
        assert_eq!(
            resolve(None, None, Some(OsStr::new("rusty-pee.exe"))),
            CompatibilityMode::Default
        );
    }

    #[test]
    fn default_when_nothing_set() {
        assert_eq!(resolve(None, None, None), CompatibilityMode::Default);
    }

    #[test]
    fn ladder_strict_flag_beats_env_var() {
        assert_eq!(
            resolve(Some(false), Some(OsStr::new("1")), None),
            CompatibilityMode::Default
        );
        assert_eq!(
            resolve(Some(true), Some(OsStr::new("0")), None),
            CompatibilityMode::Strict
        );
    }

    #[test]
    fn ladder_env_var_beats_argv0() {
        assert_eq!(
            resolve(None, Some(OsStr::new("1")), Some(OsStr::new("rusty-pee"))),
            CompatibilityMode::Strict
        );
        assert_eq!(
            resolve(None, Some(OsStr::new("0")), Some(OsStr::new("pee"))),
            CompatibilityMode::Strict,
            "rung 2 falsy is no-op; rung 3 (argv0=pee) still engages Strict"
        );
    }
}
