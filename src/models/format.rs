use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug)]
pub struct Encoder {
    pub enc: &'static str,
    pub ext: &'static str,
    pub opts: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct Format {
    pub name: &'static str,
    pub encoding: &'static str,
    pub long_name: &'static str,
    pub encoder: Encoder,
}

impl PartialEq for Format {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.encoding == other.encoding
    }
}

impl Eq for Format {}

impl Hash for Format {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.encoding.hash(state);
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.long_name)
    }
}

pub const FLAC: Format = Format {
    name: "FLAC",
    encoding: "Lossless",
    long_name: "FLAC",
    encoder: Encoder {
        enc: "flac",
        ext: ".flac",
        opts: "--best",
    },
};

pub const MP3_V0: Format = Format {
    name: "MP3",
    encoding: "V0 (VBR)",
    long_name: "MP3 V0",
    encoder: Encoder {
        enc: "lame",
        ext: ".mp3",
        opts: "-V 0 --vbr-new --ignore-tag-errors",
    },
};

pub const MP3_320: Format = Format {
    name: "MP3",
    encoding: "320",
    long_name: "MP3 320",
    encoder: Encoder {
        enc: "lame",
        ext: ".mp3",
        opts: "-h -b 320 --ignore-tag-errors",
    },
};

pub const ALL_FORMATS: [Format; 3] = [FLAC, MP3_V0, MP3_320];

pub fn format_from_name_encoding(name: &str, encoding: &str) -> Option<Format> {
    ALL_FORMATS
        .iter()
        .find(|f| f.name == name && f.encoding == encoding)
        .copied()
}
