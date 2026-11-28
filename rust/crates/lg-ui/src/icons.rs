//! Game sprites as the Java app shows them (`ImageBuffer.getOutlinedIcon`): scaled to the size less 2 (nearest
//! pixel, Java's `SCALE_DEFAULT`), with a 1 px black outline around the opaque pixels; kept as textures.

use egui::{ColorImage, TextureHandle, TextureOptions};
use lg_gamedata::GameData;
use lg_gamedata::sprites::{Image, Sprites};
use std::collections::HashMap;

pub struct Icons {
    sprites: Sprites,
    cache: HashMap<(i32, u32), Option<TextureHandle>>,
    skins: HashMap<String, TextureHandle>,
    /// Keys made before that came out empty.
    missing: std::collections::HashSet<String>,
}

/// Java's `ReplicateScaleFilter`: each target pixel takes the source pixel under its centre.
pub fn scale(im: &Image, w: u32, h: u32) -> Image {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let sy = ((2 * y * im.h + im.h) / (2 * h)).min(im.h - 1);
        for x in 0..w {
            let sx = ((2 * x * im.w + im.w) / (2 * w)).min(im.w - 1);
            let i = ((sy * im.w + sx) * 4) as usize;
            rgba.extend_from_slice(&im.rgba[i..i + 4]);
        }
    }
    Image { w, h, rgba }
}

/// The outlined icon of a sprite at a size (Java `getOutlinedIcon`).
pub fn outlined(im: &Image, size: u32) -> Image {
    let inner = scale(im, size.saturating_sub(2).max(1), size.saturating_sub(2).max(1));
    let (w, h) = (inner.w + 2, inner.h + 2);
    let alpha = |x: i64, y: i64| -> u8 {
        if x < 1 || y < 1 || x > i64::from(inner.w) || y > i64::from(inner.h) {
            return 0;
        }
        inner.rgba[(((y as u32 - 1) * inner.w + (x as u32 - 1)) * 4 + 3) as usize]
    };
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            let o = ((y as u32 * w + x as u32) * 4) as usize;
            if alpha(x, y) != 0 {
                let i = (((y as u32 - 1) * inner.w + (x as u32 - 1)) * 4) as usize;
                rgba[o..o + 4].copy_from_slice(&inner.rgba[i..i + 4]);
            } else if alpha(x + 1, y) != 0 || alpha(x - 1, y) != 0 || alpha(x, y + 1) != 0 || alpha(x, y - 1) != 0 {
                rgba[o..o + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
    Image { w, h, rgba }
}

/// The texture a dye code stands for (Java `Dyes.textureImage`): high byte 1 a solid colour, 4/5/9/10 a tile of
/// the textile sheet of that size.
fn texture_image(sprites: &mut Sprites, code: i32) -> Option<Image> {
    let (kind, value) = ((code as u32 >> 24) & 0xFF, code & 0xFF_FFFF);
    match kind {
        1 => Some(Image { w: 1, h: 1, rgba: vec![(value >> 16) as u8, (value >> 8) as u8, value as u8, 255] }),
        4 | 5 | 9 | 10 => sprites.sprite(&format!("textile{kind}x{kind}"), value),
        _ => None,
    }
}

/// Java `Dyes.dyed`: the sprite 5x, every pixel its mask marks (red: clothing, Tex1; green: accessory, Tex2) taking
/// the dye or the cloth tiled across, darkened by the mask's strength. None without a mask.
fn dyed(sprites: &mut Sprites, data: &GameData, id: i32, tex1: i32, tex2: i32) -> Option<Image> {
    const SCALE: u32 = 5;
    let src = sprites.object(data, id)?;
    let mask = sprites.mask(data, id)?;
    let (t1, t2) = (texture_image(sprites, tex1), texture_image(sprites, tex2));
    let (w, h) = (src.w * SCALE, src.h * SCALE);
    let px = |im: &Image, x: u32, y: u32| -> [u8; 4] {
        let i = ((y * im.w + x) * 4) as usize;
        [im.rgba[i], im.rgba[i + 1], im.rgba[i + 2], im.rgba[i + 3]]
    };
    let shade = |p: [u8; 4], t: [u8; 4], k: u8| -> [u8; 4] {
        let f = |c: u8| (u32::from(c) * u32::from(k) / 255) as u8;
        [f(t[0]), f(t[1]), f(t[2]), p[3]]
    };
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = (x / SCALE, y / SCALE);
            let mut p = px(&src, sx, sy);
            let m = if sx < mask.w && sy < mask.h { px(&mask, sx, sy) } else { [0; 4] };
            // Java reads the mask as ARGB: its red and green, alpha aside
            if m[0] > 0
                && let Some(t) = &t1
            {
                p = shade(p, px(t, x % t.w, y % t.h), m[0]);
            }
            if m[1] > 0
                && let Some(t) = &t2
            {
                p = shade(p, px(t, x % t.w, y % t.h), m[1]);
            }
            rgba.extend_from_slice(&p);
        }
    }
    Some(Image { w, h, rgba })
}

/// Java `Dyes.outlined`: to size-2 sampling x * w / inner, then the 1 px outline.
fn outlined_dyed(img: &Image, size: u32) -> Image {
    let inner = size.saturating_sub(2).max(1);
    let mut sampled = Image { w: inner, h: inner, rgba: Vec::with_capacity((inner * inner * 4) as usize) };
    for y in 0..inner {
        for x in 0..inner {
            let (sx, sy) = (x * img.w / inner, y * img.h / inner);
            let i = ((sy * img.w + sx) * 4) as usize;
            sampled.rgba.extend_from_slice(&img.rgba[i..i + 4]);
        }
    }
    // the same outline as getOutlinedIcon, the sampled picture at its size already
    outlined(&sampled, size)
}

/// A character's look as an image (what [`Icons::skin`] shows), for comparing with Java.
pub fn skin_image(sprites: &mut Sprites, data: &GameData, sprite: i32, tex1: i32, tex2: i32, size: u32) -> Image {
    let plain = |sprites: &mut Sprites| outlined(&sprites.object(data, sprite).unwrap_or(Image { w: 8, h: 8, rgba: vec![0; 256] }), size);
    if tex1 == 0 && tex2 == 0 {
        return plain(sprites);
    }
    match dyed(sprites, data, sprite, tex1, tex2) {
        Some(d) => outlined_dyed(&d, size),
        None => plain(sprites),
    }
}

/// The opaque part of a picture (alpha at least `min`): x, y, w, h.
fn opaque_bounds(im: &Image, min: u8) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    let mut any = false;
    for y in 0..im.h {
        for x in 0..im.w {
            if im.rgba[((y * im.w + x) * 4 + 3) as usize] >= min {
                any = true;
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    any.then(|| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

/// Nearest-pixel scaling to any size (Java's drawImage with nearest neighbour: source pixel floor(x * w / dw)).
fn resize(im: &Image, x0: u32, y0: u32, w: u32, h: u32, dw: u32, dh: u32) -> Image {
    let mut rgba = Vec::with_capacity((dw * dh * 4) as usize);
    for y in 0..dh {
        for x in 0..dw {
            let (sx, sy) = (x0 + (x * w / dw).min(w - 1), y0 + (y * h / dh).min(h - 1));
            let i = ((sy * im.w + sx) * 4) as usize;
            rgba.extend_from_slice(&im.rgba[i..i + 4]);
        }
    }
    Image { w: dw, h: dh, rgba }
}

impl Icons {
    /// Java `ActiveDungeonBadge.fit`: an object's outlined icon (at `from` px) cut to its visible pixels and scaled
    /// (crisp) into a `size` square, centred, so all portals look alike.
    pub fn fitted(&mut self, ctx: &egui::Context, data: &GameData, id: i32, from: u32, size: u32) -> Option<TextureHandle> {
        // made at the size it has on the screen (display scaling included): scaled once, every pixel the same size
        // (scaled to `size` and stretched again by the screen's factor, the pixels came out uneven)
        let px = ((size as f32 * ctx.pixels_per_point()).round() as u32).max(1);
        let key = format!("fit-{id}-{from}-{px}");
        if let Some(t) = self.skins.get(&key) {
            return Some(t.clone());
        }
        let out = self.fitted_image(data, id, from, px)?;
        let tex = ctx.load_texture(
            key.clone(),
            ColorImage::from_rgba_unmultiplied([px as usize, px as usize], &out.rgba),
            TextureOptions::NEAREST,
        );
        self.skins.insert(key, tex.clone());
        Some(tex)
    }

    /// The pixels of [`Self::fitted`].
    pub fn fitted_image(&mut self, data: &GameData, id: i32, from: u32, size: u32) -> Option<Image> {
        let src = outlined(&self.sprites.object(data, id)?, from);
        let (x0, y0, w, h) = opaque_bounds(&src, 30)?;
        let scale = f64::from(size) / f64::from(w.max(h));
        let (dw, dh) = (((f64::from(w) * scale).round() as u32).clamp(1, size), ((f64::from(h) * scale).round() as u32).clamp(1, size));
        let small = resize(&src, x0, y0, w, h, dw, dh);
        let mut out = Image { w: size, h: size, rgba: vec![0; (size * size * 4) as usize] };
        let (ox, oy) = ((size - dw) / 2, (size - dh) / 2);
        for y in 0..dh {
            for x in 0..dw {
                let s = ((y * dw + x) * 4) as usize;
                let d = (((oy + y) * size + ox + x) * 4) as usize;
                out.rgba[d..d + 4].copy_from_slice(&small.rgba[s..s + 4]);
            }
        }
        Some(out)
    }

    /// Java `DamageGraph.croppedIcon`: an enemy's outlined sprite (at 64) cut to its visible pixels and scaled (crisp)
    /// to fit a `size` square, centred on its bottom; None for a placeholder (one colour, or a plain filled square).
    pub fn cropped(&mut self, ctx: &egui::Context, data: &GameData, id: i32, size: u32) -> Option<TextureHandle> {
        let key = format!("crop-{id}-{size}");
        if let Some(t) = self.skins.get(&key) {
            return Some(t.clone());
        }
        if self.missing.contains(&key) {
            return None;
        }
        let made = (|| {
            let src = outlined(&self.sprites.object(data, id)?, 64);
            let (x0, y0, w, h) = opaque_bounds(&src, 30)?;
            let mut colors = std::collections::HashSet::new();
            let mut opaque = 0u32;
            for y in 0..src.h {
                for x in 0..src.w {
                    let i = ((y * src.w + x) * 4) as usize;
                    if src.rgba[i + 3] >= 30 {
                        opaque += 1;
                        colors.insert([src.rgba[i], src.rgba[i + 1], src.rgba[i + 2]]);
                    }
                }
            }
            if colors.len() <= 1 || colors.len() == 2 && opaque * 10 >= w * h * 9 {
                return None;
            }
            let scale = f64::from(size) / f64::from(w.max(h));
            let (dw, dh) = (((f64::from(w) * scale).round() as u32).clamp(1, size), ((f64::from(h) * scale).round() as u32).clamp(1, size));
            let small = resize(&src, x0, y0, w, h, dw, dh);
            let mut out = Image { w: size, h: size, rgba: vec![0; (size * size * 4) as usize] };
            let (ox, oy) = ((size - dw) / 2, size - dh);
            for y in 0..dh {
                for x in 0..dw {
                    let s = ((y * dw + x) * 4) as usize;
                    let d = (((oy + y) * size + ox + x) * 4) as usize;
                    out.rgba[d..d + 4].copy_from_slice(&small.rgba[s..s + 4]);
                }
            }
            Some(out)
        })();
        let Some(out) = made else {
            self.missing.insert(key);
            return None;
        };
        let tex = ctx.load_texture(
            key.clone(),
            ColorImage::from_rgba_unmultiplied([size as usize, size as usize], &out.rgba),
            TextureOptions::NEAREST,
        );
        self.skins.insert(key, tex.clone());
        Some(tex)
    }

    /// Java `DamageGraph.bigIcon`: the enemy's own cropped sprite, else (a sizeless helper, a placeholder) that of the
    /// enemy it stands for.
    pub fn enemy(&mut self, ctx: &egui::Context, data: &GameData, id: i32, size: u32) -> Option<TextureHandle> {
        if !data.enemy_sprites.hidden(id)
            && let Some(t) = self.cropped(ctx, data, id, size)
        {
            return Some(t);
        }
        for e in data.enemy_sprites.candidates(id, data.object_name(id)) {
            if let Some(t) = self.cropped(ctx, data, e, size) {
                return Some(t);
            }
        }
        None
    }

    /// A ground tile's sprite (its first texture), scaled to a size, pixel-sharp; None if unknown or empty.
    pub fn tile(&mut self, ctx: &egui::Context, data: &GameData, tile: i32, size: u32) -> Option<TextureHandle> {
        let tex = &data.tiles.get(&tile)?.texture;
        let (index, sheet) = tex.split_once(',')?;
        let sheet = sheet.split(',').next()?.to_string();
        let index: i32 = index.trim().parse().ok()?;
        let im = self.sprites.sprite(&sheet, index)?;
        if im.w <= 1 || !im.rgba.chunks(4).any(|p| p[3] > 40) {
            return None;
        }
        self.sheet(ctx, &sheet, index, size)
    }

    /// Java `DifficultyMark.plainGravestone`: a plain sprite (no outline) cut to its pixels and scaled (crisp) to a
    /// height.
    pub fn plain_cut(&mut self, ctx: &egui::Context, data: &GameData, id: i32, height: u32) -> Option<TextureHandle> {
        let key = format!("plain-{id}-{height}");
        if let Some(t) = self.skins.get(&key) {
            return Some(t.clone());
        }
        let src = self.sprites.object(data, id)?;
        let (x0, y0, w, h) = opaque_bounds(&src, 40)?;
        let height = height.max(h);
        let width = ((f64::from(w) * f64::from(height) / f64::from(h)).round() as u32).max(1);
        let out = resize(&src, x0, y0, w, h, width, height);
        let tex = ctx.load_texture(
            key.clone(),
            ColorImage::from_rgba_unmultiplied([out.w as usize, out.h as usize], &out.rgba),
            TextureOptions::NEAREST,
        );
        self.skins.insert(key, tex.clone());
        Some(tex)
    }

    /// An object's sprite as the game files have it (its native size, no outline).
    pub fn native(&mut self, data: &GameData, id: i32) -> Option<Image> {
        self.sprites.object(data, id)
    }

    /// A sprite by sheet and index scaled to a size, pixel-sharp, no outline (Java `EnchantIcons.icon`).
    /// A sheet's sprite without the green "+" an item puts in its top right corner (the Material Cap Extenders show
    /// the materials that way: without it, the material itself).
    pub fn sheet_without_plus(&mut self, ctx: &egui::Context, sheet: &str, index: i32, size: u32) -> Option<TextureHandle> {
        let key = format!("sheet-noplus-{sheet}-{index}-{size}");
        if let Some(t) = self.skins.get(&key) {
            return Some(t.clone());
        }
        let mut im = self.sprites.sprite(sheet, index)?;
        for y in 0..im.h / 2 {
            for x in im.w / 2..im.w {
                let o = ((y * im.w + x) * 4) as usize;
                let (r, g, b) = (im.rgba[o], im.rgba[o + 1], im.rgba[o + 2]);
                if g > 140 && r < 120 && b < 120 {
                    im.rgba[o + 3] = 0;
                }
            }
        }
        let out = scale(&im, size, size);
        let tex = ctx.load_texture(
            key.clone(),
            ColorImage::from_rgba_unmultiplied([out.w as usize, out.h as usize], &out.rgba),
            TextureOptions::NEAREST,
        );
        self.skins.insert(key, tex.clone());
        Some(tex)
    }

    pub fn sheet(&mut self, ctx: &egui::Context, sheet: &str, index: i32, size: u32) -> Option<TextureHandle> {
        let key = format!("sheet-{sheet}-{index}-{size}");
        if let Some(t) = self.skins.get(&key) {
            return Some(t.clone());
        }
        let im = self.sprites.sprite(sheet, index)?;
        let out = scale(&im, size, size);
        let tex = ctx.load_texture(
            key.clone(),
            ColorImage::from_rgba_unmultiplied([out.w as usize, out.h as usize], &out.rgba),
            TextureOptions::NEAREST,
        );
        self.skins.insert(key, tex.clone());
        Some(tex)
    }

    /// A character's look with its dyes (Java `CharactersView.skin` / `Dyes.icon`): the plain icon without dyes.
    pub fn skin(&mut self, ctx: &egui::Context, data: &GameData, sprite: i32, tex1: i32, tex2: i32, size: u32) -> Option<TextureHandle> {
        if tex1 == 0 && tex2 == 0 {
            return self.object(ctx, data, sprite, size);
        }
        let key = format!("skin-{sprite}-{tex1}-{tex2}-{size}");
        if let Some(t) = self.skins.get(&key) {
            return Some(t.clone());
        }
        let Some(d) = dyed(&mut self.sprites, data, sprite, tex1, tex2) else { return self.object(ctx, data, sprite, size) };
        let out = outlined_dyed(&d, size);
        let tex = ctx.load_texture(
            key.clone(),
            ColorImage::from_rgba_unmultiplied([out.w as usize, out.h as usize], &out.rgba),
            TextureOptions::NEAREST,
        );
        self.skins.insert(key, tex.clone());
        Some(tex)
    }

    /// A character wearing a set (mockup), `pieces` its weapon, ability, armor and ring:
    /// - every pixel its mask marks as clothing takes the armor's pixel at the same place (the clothing area laid over
    ///   the armor's shape), shaded as a dye would be;
    /// - the accessory takes the armor's second colour; a Knight's accessory is its shield: the ability's (the
    ///   shield's) pixels laid over it the same way;
    /// - the weapon it holds (its right edge, outside the mask, the face's colours aside) takes the weapon's colours,
    ///   darkest to brightest as its own.
    ///
    /// The plain look where the character has no mask or the armor no sprite.
    pub fn dressed(&mut self, ctx: &egui::Context, data: &GameData, sprite: i32, pieces: [i32; 4], size: u32) -> Option<TextureHandle> {
        let key = format!("dressed-{sprite}-{pieces:?}-{size}");
        if let Some(t) = self.skins.get(&key) {
            return Some(t.clone());
        }
        let [weapon, ability, armor, _] = pieces;
        let colours = self.armor_look(data, armor);
        let (Some(src), Some(mask), Some(item), Some((_, second))) =
            (self.sprites.object(data, sprite), self.sprites.mask(data, sprite), self.sprites.object(data, armor), colours)
        else {
            return self.object(ctx, data, sprite, size);
        };
        let shield = data.class(sprite).is_some_and(|c| c.name == "Knight").then(|| self.sprites.object(data, ability)).flatten();
        let weapon_colours = self.colours(data, weapon);
        let px = |im: &Image, x: u32, y: u32| -> [u8; 4] {
            let i = ((y * im.w + x) * 4) as usize;
            [im.rgba[i], im.rgba[i + 1], im.rgba[i + 2], im.rgba[i + 3]]
        };
        let bounds = |w: u32, h: u32, on: &dyn Fn(u32, u32) -> bool| {
            let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
            for y in 0..h {
                for x in 0..w {
                    if on(x, y) {
                        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                    }
                }
            }
            (x0 <= x1).then_some((x0, y0, x1, y1))
        };
        let coloured = |p: [u8; 4]| p[3] >= 128 && u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]) >= 90;
        let mask_at = |x: u32, y: u32| if x < mask.w && y < mask.h { px(&mask, x, y) } else { [0; 4] };
        let item_shape = |im: &Image| bounds(im.w, im.h, &|x, y| coloured(px(im, x, y)));
        let (Some(cloth_b), Some(armor_b)) = (bounds(src.w, src.h, &|x, y| mask_at(x, y)[0] > 0), item_shape(&item)) else {
            return self.object(ctx, data, sprite, size);
        };
        let acc_b = bounds(src.w, src.h, &|x, y| mask_at(x, y)[0] == 0 && mask_at(x, y)[1] > 0);
        let span = |a: u32, b: u32| (b - a + 1) as f32;
        // a pixel's place in `from` laid over `onto`: the pixel of `im` there
        let laid = |im: &Image, from: (u32, u32, u32, u32), onto: (u32, u32, u32, u32), x: u32, y: u32| {
            let ix = onto.0 + (((x - from.0) as f32 + 0.5) / span(from.0, from.2) * span(onto.0, onto.2)) as u32;
            let iy = onto.1 + (((y - from.1) as f32 + 0.5) / span(from.1, from.3) * span(onto.1, onto.3)) as u32;
            px(im, ix.min(onto.2), iy.min(onto.3))
        };
        let rgb = |code: i32| [(code >> 16) as u8, (code >> 8) as u8, code as u8, 255];
        let fill = rgb(colours.map_or(0, |c| c.0));
        let shade = |t: [u8; 4], a: u8, k: u8| {
            let f = |c: u8| (u32::from(c) * u32::from(k) / 255) as u8;
            [f(t[0]), f(t[1]), f(t[2]), a]
        };
        // the weapon: the right two columns' pixels outside the mask, but for the hand holding it (at the end of the
        // arm, the row whose clothing reaches the second last column)
        let is_weapon = |x: u32, y: u32| {
            let p = px(&src, x, y);
            let hand = x + 1 == src.w && src.w >= 2 && mask_at(src.w - 2, y)[0] > 0;
            x + 2 >= src.w && p[3] >= 128 && mask_at(x, y)[0] == 0 && mask_at(x, y)[1] == 0 && !hand
        };
        // its colours by how much of each it has: the most (the blade) takes the weapon's main colour, and so on
        let mut held: Vec<([u8; 3], u32)> = Vec::new();
        for y in 0..src.h {
            for x in 0..src.w {
                let p = px(&src, x, y);
                if is_weapon(x, y) {
                    match held.iter_mut().find(|(c, _)| *c == [p[0], p[1], p[2]]) {
                        Some((_, n)) => *n += 1,
                        None => held.push(([p[0], p[1], p[2]], 1)),
                    }
                }
            }
        }
        held.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        // the weapon's colours, the colourful ones (not white, grey or black) first
        let colourful = |c: [u8; 3]| c.iter().max().copied().unwrap_or(0) - c.iter().min().copied().unwrap_or(0) >= 50;
        let mut theirs = weapon_colours.clone();
        theirs.sort_by_key(|c| !colourful(*c));
        let weapon_colour = |c: [u8; 3]| -> Option<[u8; 3]> {
            let i = held.iter().position(|(h, _)| *h == c)?;
            theirs.get(i.min(theirs.len().saturating_sub(1))).copied()
        };
        let shield_b = shield.as_ref().and_then(item_shape);
        let mut rgba = Vec::with_capacity((src.w * src.h * 4) as usize);
        for y in 0..src.h {
            for x in 0..src.w {
                let p = px(&src, x, y);
                let m = mask_at(x, y);
                let out = if m[0] > 0 {
                    let ip = laid(&item, cloth_b, armor_b, x, y);
                    shade(if coloured(ip) { ip } else { fill }, p[3], m[0].max(200))
                } else if m[1] > 0 {
                    match (&shield, shield_b, acc_b) {
                        (Some(sh), Some(sb), Some(ab)) => {
                            let sp = laid(sh, ab, sb, x, y);
                            if coloured(sp) { [sp[0], sp[1], sp[2], p[3]] } else { shade(rgb(second), p[3], m[1]) }
                        }
                        _ => shade(rgb(second), p[3], m[1]),
                    }
                } else if is_weapon(x, y) {
                    weapon_colour([p[0], p[1], p[2]]).map_or(p, |c| [c[0], c[1], c[2], p[3]])
                } else {
                    p
                };
                rgba.extend_from_slice(&out);
            }
        }
        let out = outlined(&Image { w: src.w, h: src.h, rgba }, size);
        let tex = ctx.load_texture(
            key.clone(),
            ColorImage::from_rgba_unmultiplied([out.w as usize, out.h as usize], &out.rgba),
            TextureOptions::NEAREST,
        );
        self.skins.insert(key, tex.clone());
        Some(tex)
    }

    /// An item's colours, most common first, each clearly different from those before (the outline and near-black
    /// shading left out).
    pub fn colours(&mut self, data: &GameData, item: i32) -> Vec<[u8; 3]> {
        let Some(im) = self.sprites.object(data, item) else { return Vec::new() };
        let mut counts: HashMap<[u8; 3], (u32, [u32; 3])> = HashMap::new();
        for px in im.rgba.chunks(4) {
            let (r, g, b, a) = (px[0], px[1], px[2], px[3]);
            if a < 128 || u32::from(r) + u32::from(g) + u32::from(b) < 90 {
                continue;
            }
            let e = counts.entry([r / 16, g / 16, b / 16]).or_insert((0, [0; 3]));
            e.0 += 1;
            e.1[0] += u32::from(r);
            e.1[1] += u32::from(g);
            e.1[2] += u32::from(b);
        }
        let mut ranked: Vec<(u32, [u8; 3])> =
            counts.values().map(|(n, sum)| (*n, [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8])).collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let far = |a: [u8; 3], b: [u8; 3]| a.iter().zip(b).map(|(x, y)| (i32::from(*x) - i32::from(y)).abs()).sum::<i32>() > 60;
        let mut out: Vec<[u8; 3]> = Vec::new();
        for (_, c) in ranked {
            if out.iter().all(|o| far(*o, c)) {
                out.push(c);
            }
        }
        out
    }

    /// The two colours a piece of clothing is mostly made of, as dye codes (solid colours): its most common colour (for
    /// the clothing, most of a character) and the next one clearly different from it (the accessory). The black
    /// outline and near-black shading left out; None for a sprite without colour.
    pub fn armor_look(&mut self, data: &GameData, item: i32) -> Option<(i32, i32)> {
        let im = self.sprites.object(data, item)?;
        // colours in steps of 16, counted
        let mut counts: HashMap<[u8; 3], (u32, [u32; 3])> = HashMap::new();
        for px in im.rgba.chunks(4) {
            let (r, g, b, a) = (px[0], px[1], px[2], px[3]);
            if a < 128 || u32::from(r) + u32::from(g) + u32::from(b) < 90 {
                continue;
            }
            let e = counts.entry([r / 16, g / 16, b / 16]).or_insert((0, [0; 3]));
            e.0 += 1;
            e.1[0] += u32::from(r);
            e.1[1] += u32::from(g);
            e.1[2] += u32::from(b);
        }
        let mut ranked: Vec<(u32, [u8; 3])> =
            counts.values().map(|(n, sum)| (*n, [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8])).collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let main = ranked.first()?.1;
        let far = |a: [u8; 3], b: [u8; 3]| a.iter().zip(b).map(|(x, y)| (i32::from(*x) - i32::from(y)).abs()).sum::<i32>() > 90;
        let second = ranked.iter().map(|(_, c)| *c).find(|c| far(*c, main)).unwrap_or(main);
        let code = |c: [u8; 3]| 0x0100_0000 | (i32::from(c[0]) << 16) | (i32::from(c[1]) << 8) | i32::from(c[2]);
        Some((code(main), code(second)))
    }

    /// A character's look greyed out as Java shows a disabled picture (FlatLaf's dark theme `Component.grayFilter`:
    /// grey, contrast down 70 %, brightness down 20 %, the transparency kept), e.g. a lost game mode run.
    pub fn skin_faded(
        &mut self,
        ctx: &egui::Context,
        data: &GameData,
        sprite: i32,
        tex1: i32,
        tex2: i32,
        size: u32,
    ) -> Option<TextureHandle> {
        if sprite <= 0 {
            return None;
        }
        let key = format!("faded-skin-{sprite}-{tex1}-{tex2}-{size}");
        if let Some(t) = self.skins.get(&key) {
            return Some(t.clone());
        }
        let dyed_look = if tex1 == 0 && tex2 == 0 { None } else { dyed(&mut self.sprites, data, sprite, tex1, tex2) };
        let mut out = match dyed_look {
            Some(d) => outlined_dyed(&d, size),
            None => outlined(&self.sprites.object(data, sprite).unwrap_or(Image { w: 8, h: 8, rgba: vec![0; 256] }), size),
        };
        for px in out.rgba.as_chunks_mut::<4>().0 {
            let lum = 0.30 * f32::from(px[0]) + 0.59 * f32::from(px[1]) + 0.11 * f32::from(px[2]);
            let g = ((128.0 + (lum - 128.0) * 0.30) * 0.80).round().clamp(0.0, 255.0) as u8;
            px[0] = g;
            px[1] = g;
            px[2] = g;
        }
        let tex = ctx.load_texture(
            key.clone(),
            ColorImage::from_rgba_unmultiplied([out.w as usize, out.h as usize], &out.rgba),
            TextureOptions::NEAREST,
        );
        self.skins.insert(key, tex.clone());
        Some(tex)
    }

    pub fn new(assets: &std::path::Path) -> Icons {
        Icons { sprites: Sprites::new(assets), cache: HashMap::new(), skins: HashMap::new(), missing: Default::default() }
    }

    /// An object's outlined icon at a size (Java `CharactersView.sprite`), None without a sprite.
    pub fn object(&mut self, ctx: &egui::Context, data: &GameData, id: i32, size: u32) -> Option<TextureHandle> {
        if id <= 0 {
            return None;
        }
        if let Some(t) = self.cache.get(&(id, size)) {
            return t.clone();
        }
        // Java's blank 8x8 for an object without a sprite: an empty icon of the size
        let im = self.sprites.object(data, id).unwrap_or(Image { w: 8, h: 8, rgba: vec![0; 256] });
        let out = outlined(&im, size);
        let tex = ctx.load_texture(
            format!("icon-{id}-{size}"),
            ColorImage::from_rgba_unmultiplied([out.w as usize, out.h as usize], &out.rgba),
            TextureOptions::NEAREST,
        );
        self.cache.insert((id, size), Some(tex.clone()));
        Some(tex)
    }

    /// A sprite sheet's picture (`sheet`, `index`) as an outlined icon of `size`, as [`Self::object`] makes one (for
    /// an object's other texture, e.g. the seasonal vault chest).
    pub fn sprite(&mut self, ctx: &egui::Context, sheet: &str, index: i32, size: u32) -> Option<TextureHandle> {
        let key = format!("sprite-{sheet}-{index}-{size}");
        if let Some(t) = self.skins.get(&key) {
            return Some(t.clone());
        }
        let im = self.sprites.sprite(sheet, index)?;
        let out = outlined(&im, size);
        let tex = ctx.load_texture(
            key.clone(),
            ColorImage::from_rgba_unmultiplied([out.w as usize, out.h as usize], &out.rgba),
            TextureOptions::NEAREST,
        );
        self.skins.insert(key, tex.clone());
        Some(tex)
    }
}

/// Java `ProfileView.SMALL_PORTALS`: dungeons whose portal sprite is small, drawn this much bigger than their room
/// (from its centre) so they look like the others.
pub fn portal_scale(dungeon: &str) -> f32 {
    const SMALL: [&str; 6] =
        ["Plagued Nest", "Fungal Cavern", "Kogbold Steamworks", "Advanced Kogbold Steamworks", "Hidden Interregnum", "Stromwell's Rift II"];
    if SMALL.contains(&dungeon) { 1.35 } else { 1.0 }
}
