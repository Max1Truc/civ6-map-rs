use flate2::{Decompress, FlushDecompress};
use image::{ImageBuffer, Rgb, RgbImage};
use std::{cmp::min, convert::TryInto, panic};
use twoway::{find_bytes, rfind_bytes};

pub const ZLIB_HEADER: [u8; 2] = [0x78, 0x9C]; // https://stackoverflow.com/a/17176881/9438168
pub const ZLIB_START: [u8; 6] = [0x00, 0x00, 0x01, 0x00, ZLIB_HEADER[0], ZLIB_HEADER[1]];
pub const ZLIB_STOP: [u8; 4] = [0x00, 0x00, 0xFF, 0xFF];
pub const START_MAP_BUFFER: [u8; 12] = [
    0x0E, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00,
];
pub const MAP_SIZES: [(usize, (usize, usize)); 6] = [
    (1144, (44, 26)),
    (2280, (60, 38)),
    (3404, (74, 46)),
    (4536, (84, 54)),
    (5760, (96, 60)),
    (6996, (106, 66)),
];

pub fn find_zlib_buffer_indexes(buffer: &Vec<u8>, start_at_index: usize) -> Option<(usize, usize)> {
    let zlib_start_found_index = find_bytes(&buffer[start_at_index..], &ZLIB_START.to_vec());

    if zlib_start_found_index.is_some() {
        let zlib_start_index = zlib_start_found_index.unwrap() + start_at_index;
        let zlib_stop_found_index = find_bytes(&buffer[zlib_start_index..], &ZLIB_STOP.to_vec());

        if zlib_stop_found_index.is_some() {
            let zlib_stop_index = zlib_stop_found_index.unwrap() + zlib_start_index;
            return Some((zlib_start_index, zlib_stop_index));
        } else {
            return None;
        }
    } else {
        return None;
    }
}

fn find_map_start_index(data: &Vec<u8>) -> Option<usize> {
    rfind_bytes(&data, &START_MAP_BUFFER.to_vec())
}

fn extract_zlib_buffer_from_civ6_save(
    buffer: &Vec<u8>,
    output_buffer: &mut Vec<u8>,
    start_at_index: usize,
) -> Option<usize> {
    let result = find_zlib_buffer_indexes(&buffer, start_at_index);

    if result.is_none() {
        return None;
    }

    let (zlib_start_index, zlib_stop_index) = result.unwrap();
    let zlib_buffer = Vec::from(&buffer[(zlib_start_index + 4)..(zlib_stop_index + 4)]);
    output_buffer.clear();

    for x in 0..(zlib_buffer.len() / 65540 + 1) {
        // Remove some weird bits, then append them to output buffer
        let start_offset = x * 65540;
        let stop_offset = min(zlib_buffer.len(), start_offset + 65536);
        let zlib_chunk = &zlib_buffer[start_offset..stop_offset];
        output_buffer.extend_from_slice(zlib_chunk);
    }

    return Some(zlib_start_index + 1);
}

fn zlib_uncompress(compressed_data: &Vec<u8>, output_buffer: &mut Vec<u8>) {
    let mut out = output_buffer;
    let mut decompressor = Decompress::new(true);
    while decompressor.total_in() < compressed_data.len() as u64 {
        out.reserve((2usize).pow(16));
        let result = decompressor.decompress_vec(compressed_data, &mut out, FlushDecompress::Sync);

        if result.is_err() {
            break;
        }
    }
}

pub fn extract_civ6_map_data(data: &Vec<u8>, uncompressed_data: &mut Vec<u8>) {
    assert_eq!(&data[0..4], "CIV6".as_bytes());

    let mut compressed_data = Vec::new();
    let mut out = uncompressed_data;
    let mut found_map = false;
    let mut start_at_index = 0;

    while !found_map {
        let result =
            extract_zlib_buffer_from_civ6_save(&data, &mut compressed_data, start_at_index);
        if result.is_none() {
            panic!("No zlib stream containing the map was found!");
        }
        start_at_index = result.unwrap();
        out.clear();
        zlib_uncompress(&compressed_data, &mut out);
        if find_map_start_index(&out).is_some() {
            found_map = true;
        }
    }
}

pub fn tiles_number_to_max_xy(max_tiles: usize) -> (usize, usize) {
    let mut tiles_x_max = 0;
    let mut tiles_y_max = 0;

    for size in MAP_SIZES.iter() {
        if max_tiles == size.0 {
            let tiles_max = size.1;
            tiles_x_max = tiles_max.0;
            tiles_y_max = tiles_max.1;
            break;
        }
    }

    return (tiles_x_max, tiles_y_max);
}

pub fn map_render(uncompressed_data: &Vec<u8>) -> RgbImage {
    let tiles_map_start_index =
        find_map_start_index(&uncompressed_data).expect("Could not find a map in this file!");
    let tiles_number_buf =
        &uncompressed_data[(tiles_map_start_index + 12)..(tiles_map_start_index + 16)];
    let tiles_number =
        i32::from_le_bytes(tiles_number_buf.try_into().expect("Corrupted save file?"));

    let (tiles_x_max, tiles_y_max) = tiles_number_to_max_xy(tiles_number as usize);
    let mut img: RgbImage =
        ImageBuffer::new(tiles_x_max as u32 * 20 + 20, tiles_y_max as u32 * 20 + 20);

    let mut mindex = tiles_map_start_index + 16;
    for tile_n in 0..(tiles_number as usize) {
        let lengthflag1 = uncompressed_data[mindex + 51];
        let lengthflag2 = uncompressed_data[mindex + 75];
        let lengthflag3 = uncompressed_data[mindex + 49];
        let mut buflength = 55;

        if lengthflag1 & 1 != 0 {
            buflength += 24;

            if lengthflag2 & 1 != 0 {
                buflength += 20;
            }
        } else if lengthflag1 & 2 != 0 {
            buflength += 44;
        }

        if lengthflag3 & 64 != 0 {
            buflength += 17;
        }

        let tilebuf = &uncompressed_data[mindex..(mindex + buflength)];

        let x = tile_n % tiles_x_max;
        let y = tiles_y_max - tile_n / tiles_x_max - 1;

        let mut pixel_color = Rgb([128, 128, 128]);

        if lengthflag3 & 64 != 0 {
            let civ_index = tilebuf[buflength - 5];

            if civ_index == 7 {
                pixel_color = Rgb([255, 0, 0]);
            } else if civ_index == 0 {
                pixel_color = Rgb([0, 0, 255]);
            } else if civ_index == 1 {
                pixel_color = Rgb([0, 255, 0]);
            } else {
                pixel_color = Rgb([255, 255, 255]);
            }
        }

        for ox in 3..17 {
            for oy in 3..17 {
                let odd = (y as i32 % 2 - 1).abs() as u32;
                let computed_x = x as u32 * 20 + ox + 5 + 10 * odd;
                let computed_y = y as u32 * 20 + oy + 10;
                img.put_pixel(computed_x, computed_y, pixel_color);
            }
        }

        mindex += buflength;
    }

    img
}
