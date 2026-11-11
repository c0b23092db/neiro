use std::{fs, path::Path};
use lofty::prelude::*;
use lofty::picture::PictureType;

/// ジャケット画像（エンコードされたままのバイト列）
pub struct Cover {
    pub data: Vec<u8>,
    /// 拡張子（jpg / png など）
    pub ext: String,
}

/// オーディオファイルのタグ情報
#[derive(Default)]
pub struct TrackInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub cover: Option<Cover>,
}

/// ## Description
/// タグ情報とジャケットを読み込む
///
/// タグが読めないファイルでも再生は続けたいので、失敗しても空の情報を返す。
/// 埋め込みジャケットがない場合は同じフォルダの cover.jpg などを探す。
pub fn read_track_info(path:&Path) -> TrackInfo {
    let mut info = TrackInfo::default();
    if let Ok(tagged_file) = lofty::read_from_path(path) {
        if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
            info.title = non_empty(tag.title().as_deref());
            info.artist = non_empty(tag.artist().as_deref());
            info.album = non_empty(tag.album().as_deref());
        }
        // 表ジャケットを優先し、なければ最初の画像を使う
        let pictures = || tagged_file.tags().iter().flat_map(|tag| tag.pictures());
        let picture = pictures().find(|p| p.pic_type() == PictureType::CoverFront)
            .or_else(|| pictures().next());
        if let Some(picture) = picture {
            let ext = picture.mime_type().and_then(|mime| mime.ext())
                .or_else(|| guess_ext(picture.data()))
                .unwrap_or("jpg");
            info.cover = Some(Cover { data: picture.data().to_vec(), ext: ext.to_string() });
        }
    }
    if info.cover.is_none() {
        info.cover = find_cover_in_folder(path);
    }
    info
}

fn non_empty(text:Option<&str>) -> Option<String> {
    text.map(str::trim).filter(|text| !text.is_empty()).map(str::to_string)
}

fn guess_ext(data:&[u8]) -> Option<&'static str> {
    image::guess_format(data).ok()?.extensions_str().first().copied()
}

/// オーディオファイルと同じフォルダにあるジャケット画像を探す
fn find_cover_in_folder(path:&Path) -> Option<Cover> {
    const NAMES: [&str; 4] = ["cover", "folder", "front", "album"];
    const EXTS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];
    let folder = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let mut candidates:Vec<_> = fs::read_dir(folder).ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter_map(|file| {
            let stem = file.file_stem()?.to_str()?.to_lowercase();
            let ext = file.extension()?.to_str()?.to_lowercase();
            let rank = NAMES.iter().position(|name| *name == stem)?;
            EXTS.contains(&ext.as_str()).then_some((rank, ext, file))
        })
        .collect();
    candidates.sort();
    candidates.into_iter().find_map(|(_, ext, file)| {
        Some(Cover { data: fs::read(file).ok()?, ext })
    })
}
