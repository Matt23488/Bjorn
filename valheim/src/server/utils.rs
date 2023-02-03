use std::cmp::Ordering;

const HALDOR_KEY: &[u8] = &[
    0x56, 0x65, 0x6e, 0x64, 0x6f, 0x72, 0x5f, 0x42, 0x6c, 0x61, 0x63, 0x6b, 0x46, 0x6f, 0x72, 0x65,
    0x73, 0x74,
];

macro_rules! f32 {
    ($bytes:expr, $idx:expr, $offset:expr) => {{
        let offset = $idx + HALDOR_KEY.len() + $offset;
        match ($bytes[offset..offset + 4]).try_into() {
            Ok(bytes) => f32::from_le_bytes(bytes),
            Err(_) => f32::INFINITY,
        }
    }};
}

macro_rules! dist {
    ($x:expr, $y:expr) => {{
        ($x * $x + $y * $y).sqrt()
    }};
}

pub fn get_haldor_locations(world_path: &str) -> Vec<(f32, f32)> {
    match std::fs::read(world_path) {
        Ok(bytes) => {
            let mut start = 0;
            let mut locations = vec![];
            loop {
                match find_subsequence(&bytes[start..], HALDOR_KEY) {
                    Some(idx) => {
                        let idx = idx + start;

                        let x = f32!(&bytes, idx, 0);
                        let z = f32!(&bytes, idx, 8);

                        locations.push((x, z));
                        start = idx + HALDOR_KEY.len() + 12;
                    }
                    None => {
                        locations.sort_by(|(ax, az), (bx, bz)| {
                            let a_dist = dist!(ax, az);
                            let b_dist = dist!(bx, bz);
                            if a_dist < b_dist {
                                Ordering::Less
                            } else if a_dist > b_dist {
                                Ordering::Greater
                            } else {
                                Ordering::Equal
                            }
                        });

                        break locations;
                    }
                }
            }
        }
        Err(_) => vec![],
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
