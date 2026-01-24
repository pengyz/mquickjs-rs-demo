#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharKind {
    Lower,
    Upper,
    Digit,
    Other,
}

fn kind(c: char) -> CharKind {
    if c.is_ascii_lowercase() {
        CharKind::Lower
    } else if c.is_ascii_uppercase() {
        CharKind::Upper
    } else if c.is_ascii_digit() {
        CharKind::Digit
    } else {
        CharKind::Other
    }
}

fn push_token(out: &mut Vec<String>, cur: &mut String) {
    if !cur.is_empty() {
        out.push(std::mem::take(cur));
    }
}

fn split_tokens(s: &str) -> Vec<String> {
    // Split on:
    // - '_' / '-' / '.' / whitespace and any non-alnum
    // - lower->upper transitions: getName -> get + Name
    // - acronym boundary: URLValue -> URL + Value
    // - alpha<->digit transitions: Value2 -> Value + 2
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();

    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let k = kind(c);

        if k == CharKind::Other {
            push_token(&mut out, &mut cur);
            continue;
        }

        if cur.is_empty() {
            cur.push(c);
            continue;
        }

        let prev = chars[i - 1];
        let prev_k = kind(prev);

        let next = chars.get(i + 1).copied();
        let next_k = next.map(kind);

        let mut boundary = false;

        // alpha <-> digit boundary
        // For trailing digits (Value2), keep the digits in the same token to avoid `value_2`.
        if prev_k == CharKind::Digit && k != CharKind::Digit {
            boundary = true;
        } else if prev_k != CharKind::Digit && k == CharKind::Digit {
            // Don't split alpha->digit.
        }

        // lower -> upper boundary
        if prev_k == CharKind::Lower && k == CharKind::Upper {
            boundary = true;
        }

        // acronym boundary: ... U R L V ... => split before V when next is lower
        if prev_k == CharKind::Upper
            && k == CharKind::Upper
            && matches!(next_k, Some(CharKind::Lower))
        {
            boundary = true;
        }

        if boundary {
            push_token(&mut out, &mut cur);
        }

        cur.push(c);
    }

    push_token(&mut out, &mut cur);
    out
}

pub fn to_snake_case(s: &str) -> String {
    let tokens = split_tokens(s);
    let mut out = String::new();

    for t in tokens {
        let t = t.to_ascii_lowercase();
        if t.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('_');
        }
        out.push_str(&t);
    }

    if out.is_empty() {
        "_".to_string()
    } else {
        out
    }
}

pub fn to_upper_camel_case(s: &str) -> String {
    let tokens = split_tokens(s);
    let mut out = String::new();

    for t in tokens {
        if t.is_empty() {
            continue;
        }
        if t.chars().all(|c| c.is_ascii_digit()) {
            out.push_str(&t);
            continue;
        }

        // Preserve all-caps acronyms (URL/JS/etc) as a single token.
        if t.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            out.push_str(&t);
            continue;
        }

        let mut iter = t.chars();
        let Some(first) = iter.next() else {
            continue;
        };
        out.extend(first.to_uppercase());
        for ch in iter {
            out.push(ch.to_ascii_lowercase());
        }
    }

    if out.is_empty() {
        "Singleton".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_from_camel_and_acronyms() {
        assert_eq!(to_snake_case("getName"), "get_name");
        assert_eq!(to_snake_case("URLValue"), "url_value");
        assert_eq!(to_snake_case("JSValue"), "js_value");
        assert_eq!(to_snake_case("URLValue2"), "url_value2");
        assert_eq!(to_snake_case("test_fn"), "test_fn");
        assert_eq!(to_snake_case("test-fn"), "test_fn");
        assert_eq!(to_snake_case("m.testFn"), "m_test_fn");
    }

    #[test]
    fn upper_camel_case_from_snake_and_acronyms() {
        assert_eq!(to_upper_camel_case("test_fn"), "TestFn");
        assert_eq!(to_upper_camel_case("TestFn"), "TestFn");
        assert_eq!(to_upper_camel_case("getName"), "GetName");
        assert_eq!(to_upper_camel_case("URLValue"), "URLValue");
        assert_eq!(to_upper_camel_case("url_value2"), "UrlValue2");
        assert_eq!(to_upper_camel_case("m.testFn"), "MTestFn");
    }
}
