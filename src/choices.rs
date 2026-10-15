//! The playlist's options that are one word from a short list.

use serde::{Deserialize, Serialize};

/// How the list repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    /// Back to the top after the last item.
    #[default]
    All,
    /// The current item, again and again.
    One,
    /// Stop after the last item.
    Off,
}

/// What the source shows once a list that does not repeat has run out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AtEnd {
    #[default]
    Black,
    Hold,
}

/// How a picture of another shape is put on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    /// All of the picture, with bars where the shapes differ.
    #[default]
    Fit,
    /// The canvas filled, with the picture's edges cut off.
    Fill,
    /// The canvas filled, with the picture squashed or stretched.
    Stretch,
}

/// What the picture is while a sound file plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SoundPicture {
    #[default]
    Black,
    /// The last picture of whatever played before, under the music.
    Hold,
}
