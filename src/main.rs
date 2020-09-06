use std::fs::File;
use std::io::Read;

use pretty_hex::*;

const ZLIB_HEADER: [u8; 2] = [0x78, 0x9C]; // https://stackoverflow.com/a/17176881/9438168

fn main() {
    let mut file = File::open("dev_solo.Civ6Save").unwrap();
    let file_ref = Read::by_ref(&mut file);
    let mut buffer = [0; 256];

    // Read "CIV6" header
    file_ref.take(4).read(&mut buffer).unwrap();
    println!("First 4 bytes: {}", String::from_utf8_lossy(&buffer[0..4]));
    assert_eq!(&buffer[0..4], "CIV6".as_bytes());

    file_ref.take(4).read(&mut buffer).unwrap();
    let mut current_chunk: Vec<u8> = Vec::from(&buffer[0..4]);
    while file_ref.take(4).read(&mut buffer).is_ok() {
        if buffer[0..4] == [1, 0, 0, 0] {
            println!("END OF CHUNK?");
            if current_chunk.len() > 0 {
                println!("END OF CHUNK!");
                println!("{:?}\n", current_chunk.hex_dump());
                current_chunk = Vec::new();
            } else if file_ref.take(10).read(&mut buffer).is_ok() && buffer[8..9] == ZLIB_HEADER {
                // Todo: read ZLIB data
                println!("ZLIB!")
            } else {
                panic!("Error!");
            }
        } else if buffer[0..4] == [32, 0, 0, 0] {
            while file_ref.take(4).read(&mut buffer).is_ok() && buffer[0..4] != [1, 0, 0, 0] {
                current_chunk.extend_from_slice(&buffer[0..4]);
            }
            println!("Data (Version maybe?):\n{:?}\n", current_chunk.hex_dump());
            current_chunk = Vec::new();
        } else if buffer[0..4] == [2, 0, 0, 0] {
            current_chunk.extend_from_slice(&buffer[0..4]);
            file_ref.take(8).read(&mut buffer).unwrap();
            current_chunk.extend_from_slice(&buffer[0..8]);
            println!("Data (unknown type):\n{:?}\n", current_chunk.hex_dump());
            current_chunk = Vec::new();
        } else {
            // Read string
            let mut title: Vec<u8> = Vec::from(&buffer[0..4]);
            while file_ref.take(1).read(&mut buffer).is_ok() && buffer[0] != 0 {
                title.push(buffer[0]);
            }
            println!("Data ({}):", String::from_utf8_lossy(&title).to_owned());
            current_chunk = Vec::new();
            while file_ref.take(4).read(&mut buffer).is_ok() && buffer[0..4] != [1, 0, 0, 0] {
                current_chunk.extend_from_slice(&buffer[0..4]);
            }
            println!("{:?}\n", current_chunk.hex_dump());
            current_chunk = Vec::new()
        }

        current_chunk.extend_from_slice(&buffer[0..4]);
    }
}
