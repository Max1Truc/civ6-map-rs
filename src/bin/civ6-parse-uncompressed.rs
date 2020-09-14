use image::{ImageBuffer, Rgb, RgbImage};
use pretty_hex::*;
use std::{convert::TryInto, fs::File, io::Read};

const START_MAP_BUFFER: [u8; 12] = [
    0x0E, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00,
];

const MAP_SIZES: [(usize, (usize, usize)); 6] = [
    (1144, (44, 26)),
    (2280, (60, 38)),
    (3404, (74, 46)),
    (4536, (84, 54)),
    (5760, (96, 60)),
    (6996, (106, 66)),
];

fn tiles_number_to_max_xy(max_tiles: usize) -> (usize, usize) {
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

fn find_matching_buffer(
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

fn main() {
    let mut data: Vec<u8> = Vec::new();
    {
        let mut file = File::open("medicis.Civ6Save.bin").unwrap();
        file.read_to_end(&mut data).unwrap();
    }

    let tiles_map_start_index = find_matching_buffer(&data, &START_MAP_BUFFER.to_vec(), 0)
        .expect("Could not find a map in this file!");
    let tiles_number_buf = &data[(tiles_map_start_index + 12)..(tiles_map_start_index + 16)];

    let fog_start_index = 4 + find_matching_buffer(&data, &tiles_number_buf.to_vec(), 0)
        .expect("Could not find a map in this file!");
    let tiles_number =
        i32::from_le_bytes(tiles_number_buf.try_into().expect("Corrupted save file?"));

    let (tiles_x_max, tiles_y_max) = tiles_number_to_max_xy(tiles_number as usize);
    let mut img: RgbImage = ImageBuffer::new(tiles_x_max as u32 * 20, tiles_y_max as u32 * 20);

    println!("{} tiles!", tiles_number);
    println!("{} x {} tiles!", tiles_x_max, tiles_y_max);

    for tile_n in 0..(tiles_number as usize) {
        let x = tile_n % tiles_x_max;
        let y = tiles_y_max - tile_n / tiles_x_max - 1;

        let fog = data[tile_n + fog_start_index] * 255;

        let pixel_color = Rgb([fog, fog, fog]);
        for ox in 0..20 {
            for oy in 0..20 {
                img.put_pixel(x as u32 * 20 + ox, y as u32 * 20 + oy, pixel_color);
            }
        }
    }

    img.save("map.png").unwrap();
}
