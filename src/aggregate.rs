//! Exit-code aggregation across N child processes.
//!
//! Two strategies per FR-007 / FR-008:
//! - **Default mode** ([`default_max`]): `max(child_codes)` — intuitive
//!   "worst child wins" semantic.
//! - **Strict mode** ([`strict_or`]): bitwise OR over `WEXITSTATUS` — byte-equal
//!   moreutils `close_pipes()`.
//!
//! Signal-killed children contribute the code 1 (matches moreutils
//! `WIFEXITED` fallthrough — non-`WIFEXITED` `wait` result → ret |= 1).

/// Aggregate child exit codes using `max()` (Default mode, FR-007).
///
/// Empty input returns 0 (matches `--no-commands` Default-mode behavior).
#[must_use]
pub fn default_max(codes: &[i32]) -> i32 {
    codes.iter().copied().max().unwrap_or(0)
}

/// Aggregate child exit codes using bitwise OR (Strict mode, FR-008).
///
/// Matches moreutils' `ret |= WEXITSTATUS(r)` aggregation byte-for-byte for
/// the documented matrix `{(0,0)=0, (0,1)=1, (1,2)=3, (2,1)=3, (255,1)=255}`.
/// Empty input returns 0.
#[must_use]
pub fn strict_or(codes: &[i32]) -> i32 {
    codes.iter().copied().fold(0i32, |acc, c| acc | c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_max_empty_is_zero() {
        assert_eq!(default_max(&[]), 0);
    }

    #[test]
    fn default_max_single_passes_through() {
        assert_eq!(default_max(&[0]), 0);
        assert_eq!(default_max(&[1]), 1);
        assert_eq!(default_max(&[42]), 42);
        assert_eq!(default_max(&[255]), 255);
    }

    #[test]
    fn default_max_matrix() {
        // FR-007 / SC-002 matrix
        assert_eq!(default_max(&[0, 0]), 0);
        assert_eq!(default_max(&[0, 1]), 1);
        assert_eq!(default_max(&[1, 0]), 1);
        assert_eq!(default_max(&[2, 1]), 2);
        assert_eq!(default_max(&[1, 2]), 2);
        assert_eq!(default_max(&[255, 1]), 255);
    }

    #[test]
    fn strict_or_empty_is_zero() {
        assert_eq!(strict_or(&[]), 0);
    }

    #[test]
    fn strict_or_single_passes_through() {
        assert_eq!(strict_or(&[0]), 0);
        assert_eq!(strict_or(&[1]), 1);
        assert_eq!(strict_or(&[42]), 42);
        assert_eq!(strict_or(&[255]), 255);
    }

    #[test]
    fn strict_or_matrix_matches_moreutils() {
        // FR-008 / SC-003 matrix — bitwise OR, byte-equal moreutils 0.69
        assert_eq!(strict_or(&[0, 0]), 0);
        assert_eq!(strict_or(&[0, 1]), 1);
        assert_eq!(strict_or(&[1, 0]), 1);
        // Key divergence vs Default: (1,2) → OR=3, max=2
        assert_eq!(strict_or(&[2, 1]), 3);
        assert_eq!(strict_or(&[1, 2]), 3);
        assert_eq!(strict_or(&[255, 1]), 255);
    }

    #[test]
    fn modes_diverge_on_distinct_nonzero_codes() {
        // The defining behavioral divergence between modes (FR-007 vs FR-008).
        let codes = &[1, 2];
        assert_eq!(default_max(codes), 2, "Default mode: max");
        assert_eq!(strict_or(codes), 3, "Strict mode: bitwise OR");
    }
}
