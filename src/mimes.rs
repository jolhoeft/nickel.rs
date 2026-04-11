use std::str::FromStr;

macro_rules! mimes {
    ($($t:expr => { $($name:ident, $as_s:pat, $subt:expr,)+ })+) => (
        #[allow(non_camel_case_types)]
        #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
        pub enum MediaType {
            $(
                $(
                    $name
                ),*
            ),*
        }

        impl From<MediaType> for mime::Mime {
            fn from(mt: MediaType) -> mime::Mime {
                match mt {
                    $(
                        $(
                            MediaType::$name => concat!($t, "/", $subt)
                        ),*
                    ),*
                }.parse().unwrap()
            }
        }

        impl FromStr for MediaType {
            type Err = &'static str;
            fn from_str(s: &str) -> Result<MediaType, &'static str> {
                Ok(match s {
                    $(
                        $(
                            $as_s => MediaType::$name
                        ),*
                    ),*,
                    _ => return Err("Not a valid MediaType.")
                })
            }
        }
    )
}

mimes! {
    "application" => {
        Atom,     "atom",   "atom",
        Bin,      "bin",    "octet-stream",
        Bmp,      "bmp",    "bmp",
        Css,      "css",    "css",
        Dart,     "dart",   "dart",
        Dmg,      "dmg",    "apple-diskimage",
        Doc,      "doc",    "msword",
        Ear,      "ear",    "java-archive",
        Eot,      "eot",    "vnd.ms-fontobject",
        Epub,     "epub",   "epub+zip",
        Gz,       "gz",     "gzip",
        Hqx,      "hqx",    "mac-binhex40",
        Iso,      "iso",    "octet-stream",
        Jar,      "jar",    "java-archive",
        Js,       "js",     "javascript",
        Json,     "json",   "json",
        Jsonld,   "jsonld",  "ld+json",
        Kml,      "kml",    "vnd.google-earth.kml+xml",
        Kmz,      "kmz",    "vnd.google-earth.kmz",
        M3u8,     "m3u8",   "vnd.apple.mpegurl",
        Msi,      "msi",    "octet-stream",
        Msm,      "msm",    "octet-stream",
        Msp,      "msp",    "octet-stream",
        Pdf,      "pdf",    "pdf",
        Pem,      "pem",    "x-x509-ca-cert",
        Pl,       "pl",     "x-perl",
        Pm,       "pm",     "x-perl",
        Ppt,      "ppt",    "vnd.ms-powerpoint",
        Ps,       "ps",     "postscript",
        Rdf,      "rdf",    "rdf+xml",
        Rpm,      "rpm",    "x-redhat-package-manager",
        Rss,      "rss",    "rss+xml",
        Rtf,      "rtf",    "rtf",
        Sit,      "sit",    "x-stuffit",
        Swf,      "swf",    "x-shockwave-flash",
        Tar,      "tar",    "x-tar",
        Tcl,      "tcl",    "x-tcl",
        Tk,       "tk",     "x-tcl",
        Torrent,  "torrent", "x-bittorrent",
        War,      "war",    "java-archive",
        Wasm,     "wasm",   "wasm",
        Woff,     "woff",   "font-woff",
        Woff2,    "woff2",  "font-woff2",
        Xhtml,    "xhtml",  "xhtml+xml",
        Xls,      "xls",    "vnd.ms-excel",
        Xml,      "xml",    "xml",
        Xpi,      "xpi",    "x-xpinstall",
        Xspf,     "xspf",   "xspf+xml",
        Zip,      "zip",    "zip",
    }
    "audio" => {
        Aac,      "aac",    "aac",
        Flac,     "flac",   "flac",
        Kar,      "kar",    "midi",
        M4a,      "m4a",    "x-m4a",
        Mid,      "mid",    "midi",
        Midi,     "midi",   "midi",
        Mp3,      "mp3",    "mpeg",
        Ogg,      "ogg",    "ogg",
        Ra,       "ra",     "x-realaudio",
        Wav,      "wav",    "x-wav",
        Weba,     "weba",   "webm",
    }
    "image" => {
        Gif,      "gif",    "gif",
        Ico,      "ico",    "x-icon",
        Jpeg,     "jpeg",   "jpeg",
        Jpg,      "jpg",    "jpeg",
        Png,      "png",    "png",
        Svg,      "svg",    "svg+xml",
        Svgz,     "svgz",   "svg+xml",
        Tif,      "tif",    "tiff",
        Tiff,     "tiff",   "tiff",
        Wbmp,     "wbmp",   "vnd.wap.wbmp",
        Webp,     "webp",   "webp",
    }
    "text" => {
        Htm,      "htm",    "html",
        Html,     "html",   "html",
        Manifest, "manifest", "cache-manifest",
        Md,       "md",     "x-markdown",
        Mml,      "mml",    "mathml",
        Txt,      "txt",    "plain",
    }
    "video" => {
        Avi,      "avi",    "x-msvideo",
        F4v,      "f4v",    "x-f4v",
        Flv,      "flv",    "x-flv",
        Mkv,      "mkv",    "x-matroska",
        Mov,      "mov",    "quicktime",
        Mp4,      "mp4",    "mp4",
        Mpeg,     "mpeg",   "mpeg",
        Mpg,      "mpg",    "mpeg",
        Ogv,      "ogv",    "ogg",
        Ts,       "ts",     "mp2t",
        Webm,     "webm",   "webm",
        Wmv,      "wmv",    "x-ms-wmv",
    }
}
