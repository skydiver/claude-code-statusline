pub mod cache;
pub mod context;
pub mod cost;
pub mod duration;
pub mod git_branch;
pub mod model;
pub mod project;
pub mod rate_limits;
pub mod tokens;
pub mod version;

/// Format a u64 with comma thousands separators.
///
/// Matches the output of `printf "%'d"` used by the shell script for token
/// and cache counts. Handrolled to keep the crate dependency-free.
fn format_with_commas(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(b as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::format_with_commas;

    #[test]
    fn formats_small_numbers_unchanged() {
        assert_eq!(format_with_commas(0), "0");
        assert_eq!(format_with_commas(999), "999");
    }

    #[test]
    fn inserts_single_comma() {
        assert_eq!(format_with_commas(1000), "1,000");
        assert_eq!(format_with_commas(12345), "12,345");
    }

    #[test]
    fn inserts_multiple_commas() {
        assert_eq!(format_with_commas(1_234_567), "1,234,567");
        assert_eq!(format_with_commas(12_345_678_901), "12,345,678,901");
    }
}
