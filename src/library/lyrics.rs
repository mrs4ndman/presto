use std::path::Path;
use std::time::Duration;

use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedLyricLine {
    pub timestamp: Duration,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lyrics {
    Plain(String),
    Timed(Vec<TimedLyricLine>),
}

fn normalized_lyrics(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_timestamp_tag(tag: &str) -> Option<Duration> {
    let (minutes, rest) = tag.split_once(':')?;
    if minutes.is_empty() || !minutes.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    let (seconds, fraction) = match rest.split_once('.') {
        Some((seconds, fraction)) => (seconds, Some(fraction)),
        None => (rest, None),
    };

    if seconds.is_empty() || !seconds.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    let minutes: u64 = minutes.parse().ok()?;
    let seconds: u64 = seconds.parse().ok()?;
    if seconds >= 60 {
        return None;
    }

    let millis = match fraction {
        Some(fraction)
            if !fraction.is_empty() && fraction.chars().all(|ch| ch.is_ascii_digit()) =>
        {
            let digits = fraction.len().min(3);
            let value: u64 = fraction[..digits].parse().ok()?;
            match digits {
                1 => value * 100,
                2 => value * 10,
                _ => value,
            }
        }
        Some(_) => return None,
        None => 0,
    };

    Some(Duration::from_millis(
        minutes.saturating_mul(60_000) + seconds.saturating_mul(1_000) + millis,
    ))
}

fn parse_offset_tag(tag: &str) -> Option<i64> {
    let (key, value) = tag.split_once(':')?;
    if !key.eq_ignore_ascii_case("offset") {
        return None;
    }

    value.trim().parse().ok()
}

fn is_metadata_tag(tag: &str) -> bool {
    let Some((key, _)) = tag.split_once(':') else {
        return false;
    };

    matches!(
        key.trim().to_ascii_lowercase().as_str(),
        "ti" | "ar" | "al" | "au" | "by" | "re" | "ve" | "length" | "offset"
    )
}

fn split_leading_bracket_tags(line: &str) -> (Vec<&str>, &str) {
    let mut tags = Vec::new();
    let mut rest = line;

    while let Some(stripped) = rest.strip_prefix('[') {
        let Some(end) = stripped.find(']') else {
            break;
        };

        tags.push(&stripped[..end]);
        rest = &stripped[end + 1..];
    }

    (tags, rest)
}

fn apply_offset(timestamp: Duration, offset_ms: i64) -> Duration {
    if offset_ms >= 0 {
        timestamp.saturating_add(Duration::from_millis(offset_ms as u64))
    } else {
        timestamp.saturating_sub(Duration::from_millis(offset_ms.unsigned_abs()))
    }
}

pub(crate) fn parse_lyrics(text: &str) -> Option<Lyrics> {
    let plain = normalized_lyrics(text)?;

    let mut offset_ms: i64 = 0;
    let mut timed_lines: Vec<TimedLyricLine> = Vec::new();

    for raw_line in plain.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let (tags, remainder) = split_leading_bracket_tags(line);
        if tags.is_empty() {
            continue;
        }

        let mut timestamps: Vec<Duration> = Vec::new();
        let mut has_only_metadata = true;

        for tag in tags {
            if let Some(timestamp) = parse_timestamp_tag(tag.trim()) {
                timestamps.push(timestamp);
                has_only_metadata = false;
                continue;
            }

            if let Some(parsed_offset) = parse_offset_tag(tag.trim()) {
                offset_ms = parsed_offset;
                continue;
            }

            if !is_metadata_tag(tag.trim()) {
                has_only_metadata = false;
            }
        }

        if timestamps.is_empty() {
            if has_only_metadata {
                continue;
            }
            continue;
        }

        let lyric_text = remainder.trim();
        if lyric_text.is_empty() {
            continue;
        }

        for timestamp in timestamps {
            timed_lines.push(TimedLyricLine {
                timestamp: apply_offset(timestamp, offset_ms),
                text: lyric_text.to_string(),
            });
        }
    }

    if timed_lines.is_empty() {
        Some(Lyrics::Plain(plain))
    } else {
        timed_lines.sort_by_key(|line| line.timestamp);
        Some(Lyrics::Timed(timed_lines))
    }
}

fn first_non_empty_lyrics<'a>(tag: &'a Tag, key: ItemKey) -> Option<String> {
    tag.get_strings(key).find_map(normalized_lyrics)
}

pub(crate) fn lyrics_from_tag(tag: &Tag) -> Option<Lyrics> {
    first_non_empty_lyrics(tag, ItemKey::Lyrics)
        .or_else(|| first_non_empty_lyrics(tag, ItemKey::UnsyncLyrics))
        .and_then(|text| parse_lyrics(&text))
}

pub fn load_lyrics_from_path(path: &Path) -> Option<Lyrics> {
    let tagged = lofty::read_from_path(path).ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    lyrics_from_tag(tag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lofty::tag::{Tag, TagType};

    #[test]
    fn lyrics_from_tag_prefers_lyrics_key() {
        let mut tag = Tag::new(TagType::Id3v2);
        tag.insert_text(ItemKey::Lyrics, "Line 1\nLine 2".to_string());
        tag.insert_text(ItemKey::UnsyncLyrics, "Fallback".to_string());

        assert_eq!(
            lyrics_from_tag(&tag),
            Some(Lyrics::Plain("Line 1\nLine 2".to_string()))
        );
    }

    #[test]
    fn lyrics_from_tag_falls_back_to_unsynced_lyrics() {
        let mut tag = Tag::new(TagType::Id3v2);
        tag.insert_text(ItemKey::UnsyncLyrics, "Unsynced lyrics".to_string());

        assert_eq!(
            lyrics_from_tag(&tag),
            Some(Lyrics::Plain("Unsynced lyrics".to_string()))
        );
    }

    #[test]
    fn lyrics_from_tag_ignores_blank_values() {
        let mut tag = Tag::new(TagType::Id3v2);
        tag.insert_text(ItemKey::Lyrics, "   ".to_string());
        tag.insert_text(ItemKey::UnsyncLyrics, "  Actual lyrics  ".to_string());

        assert_eq!(
            lyrics_from_tag(&tag),
            Some(Lyrics::Plain("Actual lyrics".to_string()))
        );
    }

    #[test]
    fn parse_lyrics_extracts_timed_lines_and_ignores_headers() {
        let lyrics = parse_lyrics(
            "[ti:Song]\n[ar:Artist]\n[by:Provider]\n[00:10.00]first\n[00:12.50]second",
        );

        assert_eq!(
            lyrics,
            Some(Lyrics::Timed(vec![
                TimedLyricLine {
                    timestamp: Duration::from_secs(10),
                    text: "first".to_string(),
                },
                TimedLyricLine {
                    timestamp: Duration::from_millis(12_500),
                    text: "second".to_string(),
                },
            ]))
        );
    }

    #[test]
    fn parse_lyrics_applies_offset() {
        let lyrics = parse_lyrics("[offset:500]\n[00:01.00]line");

        assert_eq!(
            lyrics,
            Some(Lyrics::Timed(vec![TimedLyricLine {
                timestamp: Duration::from_millis(1_500),
                text: "line".to_string(),
            }]))
        );
    }
}
