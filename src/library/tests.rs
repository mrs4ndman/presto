use super::display::display_from_fields;
use crate::config::TrackDisplayField;
use std::path::Path;

#[test]
fn display_from_fields_can_format_artist_title() {
    let p = Path::new("/tmp/Song.mp3");
    assert_eq!(
        display_from_fields(
            p,
            "Song",
            Some("Artist"),
            None,
            &[TrackDisplayField::Artist, TrackDisplayField::Title],
            " - ",
        ),
        "Artist - Song"
    );
    assert_eq!(
        display_from_fields(
            p,
            "Song",
            Some("  Artist  "),
            None,
            &[TrackDisplayField::Artist, TrackDisplayField::Title],
            " - ",
        ),
        "Artist - Song"
    );
    assert_eq!(
        display_from_fields(
            p,
            "Song",
            None,
            None,
            &[TrackDisplayField::Artist, TrackDisplayField::Title],
            " - ",
        ),
        "Song"
    );
}
