use flate2::{Decompress, FlushDecompress, Status};
use pretty_hex::*;
use std::{
    cmp::min,
    fs::File,
    io::{Read, Write},
};

const ZLIB_HEADER: [u8; 2] = [0x78, 0x9C]; // https://stackoverflow.com/a/17176881/9438168
const ZLIB_START: [u8; 6] = [0x00, 0x00, 0x01, 0x00, ZLIB_HEADER[0], ZLIB_HEADER[1]];
const ZLIB_STOP: [u8; 4] = [0x00, 0x00, 0xFF, 0xFF];

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

fn find_zlib_buffer_indexes(buffer: &Vec<u8>) -> (usize, usize) {
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

fn extract_zlib_buffer_from_civ6_save(buffer: &Vec<u8>, output_buffer: &mut Vec<u8>) {
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

fn main() {
    let mut file = File::open("dev_solo.Civ6Save").unwrap();
    let mut data: Vec<u8> = Vec::new();
    file.read_to_end(&mut data).unwrap();

    assert_eq!(&data[0..4], "CIV6".as_bytes());

    let mut compressed_data = Vec::new();
    extract_zlib_buffer_from_civ6_save(&data, &mut compressed_data);

    let mut decompressor = Decompress::new(true);
    let mut decompressed_data: Vec<u8> = Vec::new();
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
                    decompressed_data.append(&mut temp_decompressed_data);
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

    decompressed_data.append(&mut temp_decompressed_data);

    if decompressed_data.len() > 0 {
        println!("Decompression succeeded!");
        println!("Writing result to a file...");

        let mut data_out_file = File::create("dev_solo.Civ6Save.bin").unwrap();
        data_out_file.write_all(&decompressed_data).unwrap();
    } else {
        panic!("Bad decompression! (Decompressed data is empty)");
    }
}
