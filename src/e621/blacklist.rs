/*
 * Copyright (c) 2022 McSib
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::cmp::Ordering;

use crate::e621::sender::entries::PostEntry;

/// Enum that contains each possible option from `rating:` being in blacklist.
#[derive(Debug, PartialEq, Eq)]
enum Rating {
    /// No/unrecognized rating.
    None,
    /// Safe rating.
    Safe,
    /// Questionable rating.
    Questionable,
    /// Explicit rating.
    Explicit,
}

/// Represents numeric comparison operators and ranges supported in blacklist tokens.
#[derive(Debug, PartialEq, Eq)]
enum Comparison {
    Eq(i64),
    Lt(i64),
    Lte(i64),
    Gt(i64),
    Gte(i64),
    Range(i64, i64),
    Invalid,
}

impl Comparison {
    fn matches(&self, actual: i64) -> bool {
        match *self {
            Comparison::Eq(v) => actual == v,
            Comparison::Lt(v) => actual < v,
            Comparison::Lte(v) => actual <= v,
            Comparison::Gt(v) => actual > v,
            Comparison::Gte(v) => actual >= v,
            Comparison::Range(lo, hi) => actual >= lo && actual <= hi,
            Comparison::Invalid => false,
        }
    }
}

/// The filter predicate represented by a single token in a blacklist line.
#[derive(Debug, PartialEq, Eq)]
enum FilterKind {
    Tag(String),
    Wildcard(String),
    TagCount(Comparison),
    Id(Comparison),
    Status(String),
    Rating(Rating),
    FileType(String),
    Width(Comparison),
    Height(Comparison),
    Filesize(Comparison),
    Score(Comparison),
    FavCount(Comparison),
    Fav,
    User(String),
    UserId(Comparison),
    Pool(Option<i64>),
}

impl FilterKind {
    fn matches(&self, post: &PostEntry) -> bool {
        match self {
            FilterKind::Tag(tag) => post.tags.iter_tags().any(|t| t.eq_ignore_ascii_case(tag)),
            FilterKind::Wildcard(pattern) => post
                .tags
                .iter_tags()
                .any(|t| wildcard_match(pattern, &t.to_lowercase())),
            FilterKind::TagCount(cmp) => {
                cmp.matches(i64::try_from(post.tags.iter_tags().count()).unwrap_or(i64::MAX))
            }
            FilterKind::Id(cmp) => cmp.matches(post.id),
            FilterKind::Status(status) => match status.as_str() {
                "pending" => post.flags.pending,
                "flagged" => post.flags.flagged,
                "deleted" => post.flags.deleted,
                _ => false,
            },
            FilterKind::Rating(rating) => match rating {
                Rating::Safe => post.rating.eq_ignore_ascii_case("s"),
                Rating::Questionable => post.rating.eq_ignore_ascii_case("q"),
                Rating::Explicit => post.rating.eq_ignore_ascii_case("e"),
                Rating::None => false,
            },
            FilterKind::FileType(ext) => post.file.ext.eq_ignore_ascii_case(ext),
            FilterKind::Width(cmp) => cmp.matches(post.file.width),
            FilterKind::Height(cmp) => cmp.matches(post.file.height),
            FilterKind::Filesize(cmp) => cmp.matches(post.file.size),
            FilterKind::Score(cmp) => cmp.matches(post.score.total),
            FilterKind::FavCount(cmp) => cmp.matches(post.fav_count),
            FilterKind::Fav => post.is_favorited,
            FilterKind::User(name) => post
                .uploader_name
                .as_deref()
                .is_some_and(|uploader| uploader.eq_ignore_ascii_case(name)),
            FilterKind::UserId(cmp) => cmp.matches(post.uploader_id),
            FilterKind::Pool(pool_id) => pool_id.is_some_and(|id| post.pools.contains(&id)),
        }
    }
}

/// A single token within a blacklist filter line.
#[derive(Debug, PartialEq, Eq)]
struct FilterToken {
    /// Whether this token uses the optional (`~`) OR-group prefix.
    optional: bool,
    /// Whether this token is negated (`-`).
    inverted: bool,
    /// What this token matches against.
    kind: FilterKind,
}

impl FilterToken {
    fn parse(raw: &str) -> Option<Self> {
        let lowered = raw.trim().to_lowercase();
        let mut optional = false;
        let mut inverted = false;
        let mut rest = lowered.as_str();

        while let Some(first) = rest.chars().next() {
            match first {
                '~' => {
                    optional = true;
                    rest = &rest[1..];
                }
                '-' => {
                    inverted = true;
                    rest = &rest[1..];
                }
                _ => break,
            }
        }

        if rest.is_empty() {
            return None;
        }

        let kind = if let Some((key, val)) = rest.split_once(':') {
            if let Some(kind) = parse_metatag(key, val) {
                kind
            } else {
                parse_plain_tag(rest)
            }
        } else {
            parse_plain_tag(rest)
        };

        Some(FilterToken {
            optional,
            inverted,
            kind,
        })
    }

    fn matches(&self, post: &PostEntry) -> bool {
        let matched = self.kind.matches(post);
        if self.inverted { !matched } else { matched }
    }
}

fn parse_plain_tag(tag: &str) -> FilterKind {
    if tag.contains('*') {
        FilterKind::Wildcard(tag.to_string())
    } else {
        FilterKind::Tag(tag.to_string())
    }
}

fn parse_metatag(key: &str, val: &str) -> Option<FilterKind> {
    if (key == "user" || key == "uploader")
        && let Some(stripped) = val.strip_prefix('!')
    {
        return Some(FilterKind::UserId(parse_numeric_comparison(
            stripped, parse_i64, false,
        )));
    }

    match key {
        "tagcount" => Some(FilterKind::TagCount(parse_numeric_comparison(
            val, parse_i64, false,
        ))),
        "id" => Some(FilterKind::Id(parse_numeric_comparison(
            val, parse_i64, false,
        ))),
        "status" => Some(FilterKind::Status(val.to_string())),
        "rating" => Some(FilterKind::Rating(parse_rating(val))),
        "type" => Some(FilterKind::FileType(val.to_string())),
        "width" => Some(FilterKind::Width(parse_numeric_comparison(
            val, parse_i64, false,
        ))),
        "height" => Some(FilterKind::Height(parse_numeric_comparison(
            val, parse_i64, false,
        ))),
        "filesize" => Some(FilterKind::Filesize(parse_numeric_comparison(
            val,
            parse_filesize,
            true,
        ))),
        "score" => Some(FilterKind::Score(parse_numeric_comparison(
            val, parse_i64, false,
        ))),
        "favcount" => Some(FilterKind::FavCount(parse_numeric_comparison(
            val, parse_i64, false,
        ))),
        "fav" => Some(FilterKind::Fav),
        "uploader" | "user" | "username" => Some(FilterKind::User(val.to_string())),
        "userid" => Some(FilterKind::UserId(parse_numeric_comparison(
            val, parse_i64, false,
        ))),
        "pool" => Some(FilterKind::Pool(val.parse::<i64>().ok())),
        _ => None,
    }
}

fn parse_rating(value: &str) -> Rating {
    match value {
        "safe" | "s" => Rating::Safe,
        "questionable" | "q" => Rating::Questionable,
        "explicit" | "e" => Rating::Explicit,
        _ => Rating::None,
    }
}

fn parse_i64(input: &str) -> Option<i64> {
    input.trim().parse::<i64>().ok()
}

fn is_valid_decimal(s: &str) -> bool {
    if let Some((int_part, frac_part)) = s.split_once('.') {
        !int_part.is_empty()
            && !frac_part.is_empty()
            && int_part.bytes().all(|b| b.is_ascii_digit())
            && frac_part.bytes().all(|b| b.is_ascii_digit())
    } else {
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
    }
}

fn parse_filesize(input: &str) -> Option<i64> {
    const SUFFIXES: &[(&str, f64)] = &[("mb", 1_048_576.0), ("kb", 1_024.0), ("b", 1.0)];

    let trimmed = input.trim();
    let (num_part, multiplier) = SUFFIXES
        .iter()
        .find_map(|&(suffix, mult)| trimmed.strip_suffix(suffix).map(|p| (p, mult)))
        .unwrap_or((trimmed, 1.0));

    let num_str = num_part.trim();
    if !is_valid_decimal(num_str) {
        return None;
    }

    let val: f64 = num_str.parse().ok()?;
    let bytes = (val * multiplier).floor();

    (0.0..(1u64 << 63) as f64)
        .contains(&bytes)
        .then_some(bytes as i64)
}

fn parse_numeric_comparison(
    raw: &str,
    parse_num: impl Fn(&str) -> Option<i64>,
    fuzz_equality: bool,
) -> Comparison {
    if raw.contains("..") {
        if let Some(hi) = raw.strip_prefix("..") {
            return parse_num(hi).map_or(Comparison::Invalid, Comparison::Lte);
        }
        if let Some(lo) = raw.strip_suffix("..") {
            return parse_num(lo).map_or(Comparison::Invalid, Comparison::Gte);
        }
        let mut parts = raw.split("..");
        if let (Some(lo_str), Some(hi_str), None) = (parts.next(), parts.next(), parts.next())
            && let (Some(lo), Some(hi)) = (parse_num(lo_str), parse_num(hi_str))
        {
            return Comparison::Range(lo, hi);
        }
        return Comparison::Invalid;
    }

    if let Some(num_str) = raw.strip_prefix("<=").or_else(|| raw.strip_prefix("=<")) {
        return parse_num(num_str).map_or(Comparison::Invalid, Comparison::Lte);
    }
    if let Some(num_str) = raw.strip_prefix(">=").or_else(|| raw.strip_prefix("=>")) {
        return parse_num(num_str).map_or(Comparison::Invalid, Comparison::Gte);
    }
    if let Some(num_str) = raw.strip_prefix('<') {
        return parse_num(num_str).map_or(Comparison::Invalid, Comparison::Lt);
    }
    if let Some(num_str) = raw.strip_prefix('>') {
        return parse_num(num_str).map_or(Comparison::Invalid, Comparison::Gt);
    }

    if let Some(v) = parse_num(raw) {
        if fuzz_equality {
            let lo = ((v as f64) * 0.95).trunc() as i64;
            let hi = ((v as f64) * 1.05).trunc() as i64;
            Comparison::Range(lo, hi)
        } else {
            Comparison::Eq(v)
        }
    } else {
        Comparison::Invalid
    }
}

/// Matches a wildcard pattern containing `*` (zero or more chars) literally for all other characters.
fn wildcard_match(pattern: &str, text: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    let (mut p_idx, mut t_idx) = (0, 0);
    let (mut star_idx, mut match_idx) = (None, 0);

    while t_idx < txt.len() {
        if p_idx < pat.len() && pat[p_idx] == txt[t_idx] {
            p_idx += 1;
            t_idx += 1;
        } else if p_idx < pat.len() && pat[p_idx] == '*' {
            star_idx = Some(p_idx);
            match_idx = t_idx;
            p_idx += 1;
        } else if let Some(star) = star_idx {
            p_idx = star + 1;
            match_idx += 1;
            t_idx = match_idx;
        } else {
            return false;
        }
    }

    while p_idx < pat.len() && pat[p_idx] == '*' {
        p_idx += 1;
    }

    p_idx == pat.len()
}

/// A parsed blacklist line containing required tokens (`AND`) and optional tokens (`OR`).
#[derive(Debug, Default)]
struct FilterLine {
    tokens: Vec<FilterToken>,
    optional: Vec<FilterToken>,
}

impl FilterLine {
    fn parse(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }

        let without_comment = match trimmed.find(" #") {
            Some(idx) => trimmed[..idx].trim(),
            None => trimmed,
        };
        if without_comment.is_empty() {
            return None;
        }

        let lowered = without_comment.to_lowercase();
        let mut tokens = Vec::new();
        let mut optional = Vec::new();

        for word in lowered.split_whitespace() {
            if let Some(token) = FilterToken::parse(word) {
                if token.optional {
                    optional.push(token);
                } else {
                    tokens.push(token);
                }
            }
        }

        if tokens.is_empty() && optional.is_empty() {
            return None;
        }

        Some(FilterLine { tokens, optional })
    }

    fn matches(&self, post: &PostEntry) -> bool {
        if !self.tokens.iter().all(|t| t.matches(post)) {
            return false;
        }

        if !self.optional.is_empty() && !self.optional.iter().any(|t| t.matches(post)) {
            return false;
        }

        true
    }
}

/// Blacklist that holds all of the user's parsed blacklist entries and filters posts client-side
/// following e621's blacklist syntax rules.
pub(crate) struct Blacklist {
    /// All parsed blacklist lines.
    lines: Vec<FilterLine>,
}

impl Blacklist {
    pub(crate) fn new() -> Self {
        Blacklist { lines: Vec::new() }
    }

    /// Parses the user blacklist string into filter lines.
    ///
    /// # Arguments
    ///
    /// * `user_blacklist`: The user blacklist string to parse.
    ///
    /// returns: &mut Blacklist
    pub(crate) fn parse_blacklist(&mut self, user_blacklist: &str) -> &mut Blacklist {
        trace!("Parsing blacklist...");
        self.lines = user_blacklist
            .lines()
            .filter_map(FilterLine::parse)
            .collect();
        trace!("Parsed blacklist ({} active lines)...", self.lines.len());
        self
    }

    /// Checks if the blacklist is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Returns true if the post matches any line in the blacklist.
    fn is_blacklisted(&self, post: &PostEntry) -> bool {
        self.lines.iter().any(|line| line.matches(post))
    }

    /// Filters through a set of posts, only retaining posts that aren't blacklisted.
    ///
    /// # Arguments
    ///
    /// * `posts`: Posts to filter through.
    ///
    /// returns: u16
    pub(crate) fn filter_posts(&self, posts: &mut Vec<PostEntry>) -> u16 {
        let before = posts.len();
        posts.retain(|post| !self.is_blacklisted(post));
        let filtered = u16::try_from(before.saturating_sub(posts.len())).unwrap_or(u16::MAX);

        match filtered.cmp(&1) {
            Ordering::Less => trace!("No posts filtered..."),
            Ordering::Equal => trace!("Filtered {filtered} post with blacklist..."),
            Ordering::Greater => trace!("Filtered {filtered} posts with blacklist..."),
        }

        filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::e621::sender::entries::{File, Score, Tags};

    fn make_post() -> PostEntry {
        PostEntry {
            id: 1000,
            file: File {
                width: 1920,
                height: 1080,
                ext: "png".to_string(),
                size: 1_048_576,
                ..Default::default()
            },
            score: Score {
                up: 15,
                down: -5,
                total: 10,
            },
            tags: Tags {
                general: vec!["canine".to_string(), "fur".to_string()],
                species: vec!["wolf".to_string()],
                copyright: vec!["re:zero".to_string()],
                ..Default::default()
            },
            rating: "q".to_string(),
            uploader_id: 12345,
            uploader_name: Some("SomeUploader".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_comments_negation_or_and_unknown_metatags() {
        let mut post = make_post();
        post.tags.general.push(":d".to_string());

        let mut bl = Blacklist::new();
        bl.parse_blacklist("  # comment: ignored\nwolf # inline comment: foo:bar\nunknown:foo\n:d");
        assert_eq!(bl.lines.len(), 3);
        assert!(bl.is_blacklisted(&post));

        // Unknown key:value tokens must fall back to literal tags without crashing
        bl.parse_blacklist("re:zero");
        assert!(bl.is_blacklisted(&post));
        bl.parse_blacklist("duration:>10");
        assert!(!bl.is_blacklisted(&post));

        // Multiple negations and ~ OR groups
        bl.parse_blacklist("canine -wolf -cat");
        assert!(!bl.is_blacklisted(&post));
        bl.parse_blacklist("canine -cat -dog");
        assert!(bl.is_blacklisted(&post));
        bl.parse_blacklist("rating:q ~wolf ~fox");
        assert!(bl.is_blacklisted(&post));
        bl.parse_blacklist("rating:q ~cat ~fox");
        assert!(!bl.is_blacklisted(&post));
    }

    #[test]
    fn test_numeric_ranges_and_filesize_parsing() {
        let post = make_post(); // score = 10, size = 1_048_576 (1mb)
        let mut bl = Blacklist::new();

        for (rule, expected) in [
            ("score:=<10", true),
            ("score:=>10", true),
            ("score:5..15", true),
            ("score:..10", true),
            ("score:11..", false),
            ("score:-5..10", true),
            ("score:1..2..3", false),
            ("score:abc", false),
            ("filesize:1mb", true),
            ("filesize:>=500kb", true),
            ("filesize:..1048576b", true),
            ("filesize:2mb", false),
            ("filesize:bad", false),
        ] {
            bl.parse_blacklist(rule);
            assert_eq!(
                bl.is_blacklisted(&post),
                expected,
                "failed for rule: {rule}"
            );
        }
    }

    #[test]
    fn test_wildcard_matching() {
        let mut post = make_post();
        post.tags.general.push("african_wild_dog".to_string());
        post.tags.general.push("what?".to_string());

        let mut bl = Blacklist::new();
        for (rule, expected) in [
            ("african_*", true),
            ("*_dog", true),
            ("*wild*", true),
            ("what?", true),
        ] {
            bl.parse_blacklist(rule);
            assert_eq!(bl.is_blacklisted(&post), expected, "failed for: {rule}");
        }

        // '?' is literal in e621 blacklists, not a single-char wildcard
        post.tags.general.pop();
        post.tags.general.push("whatx".to_string());
        bl.parse_blacklist("what?");
        assert!(!bl.is_blacklisted(&post));
    }

    #[test]
    fn test_user_and_userid_shorthand() {
        let post = make_post();
        let mut bl = Blacklist::new();

        for rule in [
            "user:someuploader",
            "uploader:SomeUploader",
            "username:someuploader",
            "userid:12345",
            "user:!12345",
            "uploader:!12345",
            "userid:12000..13000",
        ] {
            bl.parse_blacklist(rule);
            assert!(bl.is_blacklisted(&post), "failed for rule: {rule}");
        }
    }
}
