use image_dds::ddsfile::Dds;
use image_dds::Surface;
use std::collections::HashMap;

pub fn sat_val(rgb: Rgb8) -> (f32, f32) {
    let [r, g, b] = rgb.map(|c| c as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let s = if max > 0.0 { (max - min) / max } else { 0.0 };
    (s, max)
}

pub fn pop_score(rgb: Rgb8) -> f32 {
    let (s, v) = sat_val(rgb);
    let v_penalty = 1.0 - (v - 0.5).abs() * 0.6;
    s * v_penalty.max(0.0)
}

pub fn bucket_key(rgb: Rgb8) -> (u8, u8, u8) {
    (rgb[0] >> 4, rgb[1] >> 4, rgb[2] >> 4)
}

pub fn spread_to_confidence(spread: i32) -> f32 {
    1.0 - (spread.clamp(0, 765) as f32 / 765.0)
}

pub type Rgb8 = [u8; 3];

#[derive(Clone, Debug, PartialEq)]
pub struct PrimaryColors {
    pc: Box<[Box<[(Rgb8, f32)]>]>,
    block_size: u32,
}

impl PrimaryColors {
    pub fn get(&self, x: u32, y: u32) -> Option<(Rgb8, f32)> {
        let x = (x / self.block_size) as usize;
        let y = (y / self.block_size) as usize;

        if self.pc.len() <= y || self.pc[0].len() <= x {
            return None;
        }
        Some(self.pc[y][x])
    }

    pub fn dominant_pop_color(&self, x_min: u32, y_min: u32, x_max: u32, y_max: u32) -> Option<Rgb8> {
        let mut buckets: HashMap<(u8, u8, u8), (u32, [u32; 3], f32)> = HashMap::new();
        let block_size = self.block_size;

        let mut y = y_min;
        while y < y_max {
            let mut x = x_min;
            while x < x_max {
                if let Some((rgb, confidence)) = self.get(x, y) {
                    let key = bucket_key(rgb);
                    let entry = buckets.entry(key).or_insert((0, [0, 0, 0], 0.0));
                    entry.0 += 1;
                    entry.1[0] += rgb[0] as u32;
                    entry.1[1] += rgb[1] as u32;
                    entry.1[2] += rgb[2] as u32;
                    entry.2 += pop_score(rgb) * confidence;
                }
                x += block_size;
            }
            y += block_size;
        }

        let pick = |min_count: u32| {
            buckets
                .values()
                .filter(|(count, ..)| *count >= min_count)
                .max_by(|a, b| (a.2 / a.0 as f32).total_cmp(&(b.2 / b.0 as f32)))
                .map(|(count, sum, _)| {
                    [
                        (sum[0] / count) as u8,
                        (sum[1] / count) as u8,
                        (sum[2] / count) as u8,
                    ]
                })
        };

        pick(2).or_else(|| pick(1))
    }

    pub fn block_average(data: &[u8], width: usize, x0: usize, y0: usize, x1: usize, y1: usize) -> (Rgb8, f32) {
        let mut sum = [0u32; 3];
        let mut min = [255u8; 3];
        let mut max = [0u8; 3];
        let mut count = 0u32;

        for y in y0..y1 {
            for x in x0..x1 {
                let idx = (y * width + x) * 4;
                let px = [data[idx], data[idx + 1], data[idx + 2]];
                for c in 0..3 {
                    sum[c] += px[c] as u32;
                    min[c] = min[c].min(px[c]);
                    max[c] = max[c].max(px[c]);
                }
                count += 1;
            }
        }

        if count == 0 {
            return ([0, 0, 0], 0.0);
        }

        let avg = [
            (sum[0] / count) as u8,
            (sum[1] / count) as u8,
            (sum[2] / count) as u8,
        ];
        let spread = (max[0] as i32 - min[0] as i32)
            + (max[1] as i32 - min[1] as i32)
            + (max[2] as i32 - min[2] as i32);

        (avg, spread_to_confidence(spread))
    }

    pub fn build_from_rgba(width: usize, height: usize, data: &[u8]) -> Self {
        const BLOCK: usize = 4;
        let blocks_wide = (width + BLOCK - 1) / BLOCK;
        let blocks_high = (height + BLOCK - 1) / BLOCK;

        let mut pc: Vec<Box<[(Rgb8, f32)]>> = Vec::with_capacity(blocks_high);
        for by in 0..blocks_high {
            let mut row: Vec<(Rgb8, f32)> = Vec::with_capacity(blocks_wide);
            for bx in 0..blocks_wide {
                let x0 = bx * BLOCK;
                let y0 = by * BLOCK;
                let x1 = (x0 + BLOCK).min(width);
                let y1 = (y0 + BLOCK).min(height);
                row.push(Self::block_average(data, width, x0, y0, x1, y1));
            }
            pc.push(row.into_boxed_slice());
        }

        Self {
            pc: pc.into_boxed_slice(),
            block_size: BLOCK as u32,
        }
    }
}

impl TryFrom<&'_ Dds> for PrimaryColors {
    type Error = ();

    fn try_from(dds: &'_ Dds) -> Result<Self, Self::Error> {
        let surface = Surface::from_dds(dds).map_err(|_| ())?.decode_rgba8().map_err(|_| ())?;
        Ok(Self::build_from_rgba(surface.width as usize, surface.height as usize, &surface.data))
    }
}