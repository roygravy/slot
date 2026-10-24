use scraper::{Html, Selector};
use slot_store::{Cart, Platform};
use unicode_normalization::UnicodeNormalization;

use crate::{Error, Transport};
const BASE: &str = "https://gamesdb.launchbox-app.com";
pub(crate) const IMAGE: &str = "https://images.launchbox-app.com/";

pub(crate) fn allowed_url(url: &str) -> bool {
    url.starts_with(&format!("{BASE}/")) || url.starts_with(IMAGE)
}

fn unavailable(message: &str) -> Error {
    Error::Unavailable(message.into())
}

fn selector(s: &str) -> Selector {
    Selector::parse(s).expect("static CSS selector")
}

pub fn title(stem: &str) -> String {
    let mut depth = 0u32;
    let clean: String = stem
        .chars()
        .filter(|c| match c {
            '(' | '[' => {
                depth += 1;
                false
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                false
            }
            _ => depth == 0,
        })
        .collect();
    normalize(&clean.replace(", The", ""))
}

pub fn normalize(value: &str) -> String {
    let mut text = String::new();
    for c in value
        .nfkd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
    {
        match c {
            '&' => text.push_str(" and "),
            '$' => text.push('s'),
            '\'' | '’' => {}
            c if c.is_alphanumeric() => text.extend(c.to_lowercase()),
            _ => text.push(' '),
        }
    }
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    text.strip_prefix("the ")
        .or_else(|| text.strip_prefix("disneys "))
        .or_else(|| text.strip_prefix("lara croft "))
        .unwrap_or(&text)
        .to_owned()
}

fn encode(query: &str) -> String {
    let mut result = String::new();
    for b in query.bytes() {
        if b.is_ascii_alphanumeric() {
            result.push(b as char);
        } else {
            result.push_str(&format!("%{b:02X}"));
        }
    }
    result
}

/// The source's names for a cart's platform, its own first. Game Boy and Game Boy Color share
/// ROM extensions, so a cart may sit on either shelf.
pub fn platforms(platform: Platform) -> &'static [&'static str] {
    match platform {
        Platform::Gba => &["Nintendo Game Boy Advance"],
        Platform::Gb => &["Nintendo Game Boy", "Nintendo Game Boy Color"],
        Platform::Gbc => &["Nintendo Game Boy Color", "Nintendo Game Boy"],
        Platform::Nes => &["Nintendo Entertainment System"],
        Platform::Snes => &["Super Nintendo Entertainment System"],
    }
}

pub(crate) fn game_id(page: &str, wanted: &str, platform: Platform) -> Result<u64, Error> {
    let doc = Html::parse_document(page);
    for name in platforms(platform) {
        let found = ids(&doc, wanted, name);
        match found.as_slice() {
            [] => continue,
            [(id, _)] => return Ok(*id),
            // Hacks and remakes share a title with the game ("Super Mario Bros. 3 +"): the game
            // is the one far more people voted on, when one is.
            _ => {
                let most = found.iter().map(|(_, v)| *v).max().unwrap_or(0);
                let leaders: Vec<u64> = found.iter().filter(|(_, v)| *v == most).map(|(id, _)| *id).collect();
                match leaders.as_slice() {
                    [id] if most > 0 => return Ok(*id),
                    _ => break,
                }
            }
        }
    }
    Err(unavailable("no unique title match"))
}

/// A game a search found: its id, its name, and the name of its platform as people say it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Game {
    pub id: u64,
    pub name: String,
    pub platform: String,
}

/// The most games a search lists.
const GAMES: usize = 8;

/// A shelf's platform as people say it, for a source's name of it.
fn spoken(platform: &str) -> &str {
    match platform {
        "Nintendo Game Boy" => "Game Boy",
        "Nintendo Game Boy Color" => "Game Boy Color",
        "Nintendo Game Boy Advance" => "Game Boy Advance",
        "Nintendo Entertainment System" => "NES",
        "Super Nintendo Entertainment System" => "Super Nintendo",
        other => other,
    }
}

/// The games a results page lists for the shelf's platforms, the shelf's own first and each
/// in the page's order.
pub(crate) fn games(page: &str, platform: Platform) -> Vec<Game> {
    let doc = Html::parse_document(page);
    let mut found: Vec<(usize, Game)> = Vec::new();
    for link in doc.select(&selector("a[href^='/games/details/']")) {
        let Some(heading) = link.select(&selector("h3")).next() else {
            continue;
        };
        let name = heading.text().collect::<String>().trim().to_owned();
        let texts: Vec<String> = link.select(&selector("p")).map(|p| p.text().collect()).collect();
        let Some((rank, listed)) = platforms(platform)
            .iter()
            .enumerate()
            .find(|(_, p)| texts.iter().any(|t| t.trim() == **p))
        else {
            continue;
        };
        let id = link
            .value()
            .attr("href")
            .and_then(|h| h.strip_prefix("/games/details/"))
            .and_then(|s| s.split('-').next())
            .and_then(|s| s.parse::<u64>().ok());
        if let Some(id) = id.filter(|id| !found.iter().any(|(_, g)| g.id == *id)) {
            let platform = spoken(listed).to_owned();
            found.push((rank, Game { id, name, platform }));
        }
    }
    found.sort_by_key(|(rank, _)| *rank);
    found.into_iter().take(GAMES).map(|(_, g)| g).collect()
}

/// The games a search for the words typed finds on the shelf's platforms.
pub(crate) fn search(
    http: &mut impl Transport,
    query: &str,
    platform: Platform,
) -> Result<Vec<Game>, Error> {
    let query = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if query.is_empty() {
        return Err(unavailable("nothing to search for"));
    }
    let page = http.get(&format!("{BASE}/games/results?id={}", encode(&query)))?;
    Ok(games(&String::from_utf8_lossy(&page), platform))
}

/// The games listed under this title and platform, each with its count of votes.
fn ids(doc: &Html, wanted: &str, platform: &str) -> Vec<(u64, u32)> {
    let mut found: Vec<(u64, u32)> = Vec::new();
    for link in doc.select(&selector("a[href^='/games/details/']")) {
        let Some(heading) = link.select(&selector("h3")).next() else {
            continue;
        };
        let name = heading.text().collect::<String>();
        let texts: Vec<String> = link.select(&selector("p")).map(|p| p.text().collect()).collect();
        let listed = texts.iter().any(|t| t.trim() == platform);
        if !listed || normalize(&name) != wanted {
            continue;
        }
        let votes = texts
            .iter()
            .find_map(|t| t.split("Total Votes:").nth(1))
            .and_then(|v| v.trim().split_whitespace().next()?.replace(',', "").parse().ok())
            .unwrap_or(0);
        let href = link.value().attr("href").unwrap_or_default();
        if let Some(id) = href
            .strip_prefix("/games/details/")
            .and_then(|s| s.split('-').next())
            .and_then(|s| s.parse::<u64>().ok())
        {
            if !found.iter().any(|(f, _)| *f == id) {
                found.push((id, votes));
            }
        }
    }
    found
}

fn known(title: &str) -> Option<u64> {
    Some(match title {
        "advance wars" => 2367,
        "aladdin" => 3215,
        "dexters laboratory deesaster strikes" => 26190,
        "lady sia" => 14182,
        "legend of zelda a link to the past and four swords" => 8376,
        "metal slug advance" => 10743,
        "metroid zero mission" => 3552,
        "super mario advance" => 2225,
        "tetris worlds" => 3827,
        "tom clancys rainbow six rogue spear" => 3412,
        "warioware inc mega microgames" => 3917,
        _ => return None,
    })
}

fn regions(stem: &str, code: &str) -> &'static [&'static str] {
    let stem = stem.to_lowercase();
    // Filename regions take precedence over a possibly shared header code.
    let tags: Vec<_> = stem
        .split('(')
        .skip(1)
        .filter_map(|s| s.split_once(')').map(|p| p.0))
        .collect();
    for (token, regions) in [
        ("usa", &["North America", "United States"][..]),
        ("europe", &["Europe"][..]),
        ("japan", &["Japan"][..]),
        ("australia", &["Australia", "Oceania"][..]),
    ] {
        if tags
            .iter()
            .any(|tag| tag.split(',').any(|s| s.trim() == token))
        {
            return regions;
        }
    }
    match code.as_bytes().get(3) {
        Some(b'J') => &["Japan"],
        Some(b'P') | Some(b'D') | Some(b'F') | Some(b'I') | Some(b'S') => &["Europe"],
        _ => &["North America", "United States"],
    }
}

pub const PHOTO: &str = "Cart - Front";
pub const RENDER: &str = "Fanart - Cart - Front";

/// Cart fronts, best first and at most three: a photo of the cart from its own region, a fanart
/// render of that cart, then the same from any other region. A render is a scan of the real label
/// on a stock shell, and for Game Boy and Game Boy Color it is most of what LaunchBox has.
pub fn rank<T: Clone>(stem: &str, code: &str, images: &[(&str, &str, T)]) -> Vec<T> {
    rank_all(stem, code, images)
        .into_iter()
        .take(3)
        .map(|(_, _, image)| image)
        .collect()
}

/// Every cart front in the same order, each with whether it is a render and its region.
fn rank_all<T: Clone>(stem: &str, code: &str, images: &[(&str, &str, T)]) -> Vec<(bool, String, T)> {
    let own = regions(stem, code);
    let mut ranked: Vec<_> = images
        .iter()
        .filter_map(|(kind, region, image)| {
            let render = match *kind {
                PHOTO => false,
                RENDER => true,
                _ => return None,
            };
            let away = !own.contains(region);
            Some((
                u8::from(away) * 2 + u8::from(render),
                (render, region.to_string(), image.clone()),
            ))
        })
        .collect();
    ranked.sort_by_key(|(tier, _)| *tier);
    ranked.into_iter().map(|(_, front)| front).collect()
}

/// A gallery's cart fronts. Titles read "<game> - <type> Image (<region>)", the region optional.
pub(crate) fn image_urls(page: &str, cart: &Cart) -> Result<Vec<String>, Error> {
    match fronts(page, cart) {
        fronts if fronts.is_empty() => Err(unavailable("no cartridge artwork")),
        fronts => Ok(fronts.into_iter().take(3).map(|(_, _, url)| url).collect()),
    }
}

/// A gallery's cart fronts, all of them, best first: whether each is a render, its region and
/// its address.
fn fronts(page: &str, cart: &Cart) -> Vec<(bool, String, String)> {
    let doc = Html::parse_document(page);
    let mut images = Vec::new();
    for link in doc.select(&selector("a[data-title][href]")) {
        let description = link.value().attr("data-title").unwrap_or_default();
        let url = link.value().attr("href").unwrap_or_default();
        let Some((head, tail)) = description.rsplit_once(" Image") else {
            continue;
        };
        let region = tail
            .trim()
            .strip_prefix('(')
            .and_then(|r| r.strip_suffix(')'))
            .unwrap_or_default();
        let kind = [RENDER, PHOTO]
            .into_iter()
            .find(|kind| head.ends_with(&format!(" - {kind}")));
        if let Some(kind) = kind.filter(|_| url.starts_with(IMAGE)) {
            images.push((kind, region, url.to_owned()));
        }
    }
    rank_all(&cart.stem, &cart.code, &images)
}

pub(crate) fn resolve(
    http: &mut impl Transport,
    cart: &Cart,
    canonical_title: Option<&str>,
) -> Result<Vec<String>, Error> {
    let page = gallery(http, cart, canonical_title)?;
    image_urls(&page, cart)
}

/// Every cart front of the game's gallery, best first: whether each is a render, its region
/// and its address.
pub(crate) fn all_fronts(
    http: &mut impl Transport,
    cart: &Cart,
) -> Result<Vec<(bool, String, String)>, Error> {
    let page = gallery(http, cart, None)?;
    match fronts(&page, cart) {
        fronts if fronts.is_empty() => Err(unavailable("no cartridge artwork")),
        fronts => Ok(fronts),
    }
}

/// Every cart front of a game picked from a search, best first.
pub(crate) fn fronts_of(
    http: &mut impl Transport,
    cart: &Cart,
    id: u64,
) -> Result<Vec<(bool, String, String)>, Error> {
    let page = http.get(&format!("{BASE}/games/images/{id}"))?;
    match fronts(&String::from_utf8_lossy(&page), cart) {
        fronts if fronts.is_empty() => Err(unavailable("no cartridge artwork")),
        fronts => Ok(fronts),
    }
}

/// The game's image gallery, found by its title.
fn gallery(
    http: &mut impl Transport,
    cart: &Cart,
    canonical_title: Option<&str>,
) -> Result<String, Error> {
    let title = canonical_title
        .map(normalize)
        .unwrap_or_else(|| title(&cart.stem));
    if title.is_empty() {
        return Err(unavailable("empty game title"));
    }
    let id = match Some(&title)
        .filter(|_| cart.platform == Platform::Gba)
        .and_then(|t| known(t))
    {
        Some(id) => id,
        None => {
            let page = http.get(&format!("{BASE}/games/results?id={}", encode(&title)))?;
            game_id(&String::from_utf8_lossy(&page), &title, cart.platform)?
        }
    };
    let page = http.get(&format!("{BASE}/games/images/{id}"))?;
    Ok(String::from_utf8_lossy(&page).into_owned())
}
