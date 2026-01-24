use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    pub fn parse_no_ws(s: &str) -> Option<Self> {
        if s.is_empty() || s.bytes().any(|b| b.is_ascii_whitespace()) {
            return None;
        }

        let mut it = s.split('.');
        let major = it.next()?.parse::<u16>().ok()?;

        let minor = match it.next() {
            Some(x) => x.parse::<u16>().ok()?,
            None => 0,
        };

        let patch = match it.next() {
            Some(x) => x.parse::<u16>().ok()?,
            None => 0,
        };

        if it.next().is_some() {
            return None;
        }

        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionOp {
    Eq,
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequireSpec {
    Latest {
        base: String,
    },
    Exact {
        base: String,
        version: Version,
    },
    Range {
        base: String,
        op: VersionOp,
        version: Version,
    },
}

/// Parse require(spec) argument.
///
/// Grammar (V1):
/// - `<base>`
/// - `<base>@<version>`
/// - `<base>@<op><version>` where op in {>,>=,<,<=}
///
/// Constraints:
/// - No whitespace anywhere.
/// - base must not contain '@'.
pub fn parse_require_spec_no_ws(spec: &str) -> Option<RequireSpec> {
    if spec.is_empty() || spec.bytes().any(|b| b.is_ascii_whitespace()) {
        return None;
    }

    let (base, tail) = match spec.split_once('@') {
        None => {
            return Some(RequireSpec::Latest {
                base: spec.to_string(),
            })
        }
        Some((b, t)) => (b, t),
    };

    if base.is_empty() || tail.is_empty() {
        return None;
    }

    if base.contains('@') {
        return None;
    }

    let (op, ver_str) = if let Some(rest) = tail.strip_prefix(">=") {
        (VersionOp::Ge, rest)
    } else if let Some(rest) = tail.strip_prefix("<=") {
        (VersionOp::Le, rest)
    } else if let Some(rest) = tail.strip_prefix('>') {
        (VersionOp::Gt, rest)
    } else if let Some(rest) = tail.strip_prefix('<') {
        (VersionOp::Lt, rest)
    } else {
        (VersionOp::Eq, tail)
    };

    let version = Version::parse_no_ws(ver_str)?;

    match op {
        VersionOp::Eq => Some(RequireSpec::Exact {
            base: base.to_string(),
            version,
        }),
        _ => Some(RequireSpec::Range {
            base: base.to_string(),
            op,
            version,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_latest() {
        assert_eq!(
            parse_require_spec_no_ws("system.network"),
            Some(RequireSpec::Latest {
                base: "system.network".to_string()
            })
        );
    }

    #[test]
    fn parse_exact() {
        assert_eq!(
            parse_require_spec_no_ws("system.network@1.2"),
            Some(RequireSpec::Exact {
                base: "system.network".to_string(),
                version: Version {
                    major: 1,
                    minor: 2,
                    patch: 0
                }
            })
        );
    }

    #[test]
    fn parse_ranges() {
        assert_eq!(
            parse_require_spec_no_ws("system.network@>1.2"),
            Some(RequireSpec::Range {
                base: "system.network".to_string(),
                op: VersionOp::Gt,
                version: Version {
                    major: 1,
                    minor: 2,
                    patch: 0
                }
            })
        );

        assert_eq!(
            parse_require_spec_no_ws("system.network@>=1"),
            Some(RequireSpec::Range {
                base: "system.network".to_string(),
                op: VersionOp::Ge,
                version: Version {
                    major: 1,
                    minor: 0,
                    patch: 0
                }
            })
        );

        assert_eq!(
            parse_require_spec_no_ws("system.network@<1.2.3"),
            Some(RequireSpec::Range {
                base: "system.network".to_string(),
                op: VersionOp::Lt,
                version: Version {
                    major: 1,
                    minor: 2,
                    patch: 3
                }
            })
        );

        assert_eq!(
            parse_require_spec_no_ws("system.network@<=1.2.3"),
            Some(RequireSpec::Range {
                base: "system.network".to_string(),
                op: VersionOp::Le,
                version: Version {
                    major: 1,
                    minor: 2,
                    patch: 3
                }
            })
        );
    }

    #[test]
    fn reject_whitespace_and_bad_forms() {
        for s in [
            "",
            " ",
            "system.network@ 1.0",
            "system.network @1.0",
            "system.network@>= 1.0",
            "system.network@",
            "@1.0",
            "system.network@1.2.3.4",
        ] {
            assert_eq!(parse_require_spec_no_ws(s), None, "{s}");
        }
    }
}
