use std::{env, fs::File, io::Read};

mod lib;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 3 {
        let mut uncompressed_data = Vec::new();
        {
            let mut base_save_data: Vec<u8> = Vec::new();
            {
                let mut file = File::open(&args[1]).unwrap();
                file.read_to_end(&mut base_save_data).unwrap();
            }
            lib::extract_civ6_compressed_data(&base_save_data, &mut uncompressed_data);
        }

        if uncompressed_data.len() > 0 {
            // Decompression succeeded
            // We render the map
            let img = lib::map_render(&uncompressed_data);
            img.save(&args[2]).unwrap();
        } else {
            panic!("Bad decompression! (Decompressed data is empty)");
        }
    } else {
        println!(
            "Usage:\n  {} [path to .Civ6Save file] [name of the output image]",
            args[0]
        );
    }
}
