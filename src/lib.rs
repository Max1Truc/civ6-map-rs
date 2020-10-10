use flate2::{Decompress, FlushDecompress, Status};
use image::{ImageBuffer, Rgb, RgbImage};
use std::{cmp::min, convert::TryInto};

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

pub fn find_matching_buffer(
    base_buffer: &Vec<u8>,
    sub_buffer: &Vec<u8>,
    start_at_index: usize,
) -> Option<usize> {
    let mut matching_bytes = 0;
    let mut index = 0;

    let start_index = base_buffer.iter().position(|e| {
        if index < start_at_index {
            index += 1;
            return false;
        }

        index += 1;

        if sub_buffer[matching_bytes] == *e {
            matching_bytes += 1;

            if matching_bytes == sub_buffer.len() {
                return true;
            }
        } else {
            matching_bytes = 0;
        }
        return false;
    });

    if start_index.is_some() {
        return Some(start_index.unwrap() + 1 - sub_buffer.len());
    }

    None
}

pub fn find_zlib_buffer_indexes(buffer: &Vec<u8>) -> (usize, usize) {
    let zlib_start_index = find_matching_buffer(buffer, &ZLIB_START.to_vec(), 0);

    if zlib_start_index.is_some() {
        let zlib_start_index = zlib_start_index.unwrap();
        let zlib_stop_index = find_matching_buffer(buffer, &ZLIB_STOP.to_vec(), zlib_start_index);

        if zlib_stop_index.is_some() {
            let zlib_stop_index = zlib_stop_index.unwrap();
            return (zlib_start_index, zlib_stop_index);
        } else {
            panic!("No bytes matching end of zlib stream!");
        }
    } else {
        panic!("No bytes matching start of zlib stream!");
    }
}

pub fn extract_zlib_buffer_from_civ6_save(buffer: &Vec<u8>, output_buffer: &mut Vec<u8>) {
    let (zlib_start_index, zlib_stop_index) = find_zlib_buffer_indexes(&buffer);

    let zlib_buffer = Vec::from(&buffer[(zlib_start_index + 4)..(zlib_stop_index + 4)]);
    output_buffer.clear();

    for x in 0..(zlib_buffer.len() / 65540 + 1) {
        let start_offset = x * 65540;
        let stop_offset = min(zlib_buffer.len(), start_offset + 65536);
        let zlib_chunk = &zlib_buffer[start_offset..stop_offset];
        output_buffer.extend_from_slice(zlib_chunk);
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

pub fn extract_civ6_compressed_data(data: &Vec<u8>, output_buffer: &mut Vec<u8>) {
    assert_eq!(&data[0..4], "CIV6".as_bytes());

    let mut compressed_data = Vec::new();
    extract_zlib_buffer_from_civ6_save(&data, &mut compressed_data);

    let mut decompressor = Decompress::new(true);
    let mut temp_decompressed_data: Vec<u8> = Vec::with_capacity((1024 as usize).pow(2)); // 1 mb is read each time
    loop {
        if decompressor.total_in() as usize == compressed_data.len() {
            break;
        }

        let remaining_data = &compressed_data[(decompressor.total_in() as usize)..];
        let result = decompressor.decompress_vec(
            remaining_data,
            &mut temp_decompressed_data,
            FlushDecompress::Sync,
        );

        if result.is_ok() {
            let status = result.unwrap();
            match status {
                Status::Ok => {
                    output_buffer.append(&mut temp_decompressed_data);
                }
                Status::StreamEnd => {
                    break;
                }
                Status::BufError => panic!("Error with buffer! Corrupted data?"),
            }
        } else {
            panic!("{:?}", result);
        }
    }

    output_buffer.append(&mut temp_decompressed_data);
}

pub fn map_render(uncompressed_data: &Vec<u8>) -> RgbImage {
    let tiles_map_start_index =
        find_matching_buffer(&uncompressed_data, &START_MAP_BUFFER.to_vec(), 0)
            .expect("Could not find a map in this file!");
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

            println!("{}", civ_index);

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
