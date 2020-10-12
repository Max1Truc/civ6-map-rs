use std::{
    env,
    ffi::OsStr,
    fs::File,
    io::{Read, Write},
    path::Path,
};

mod lib;

fn usage(args: &Vec<String>) {
    println!(
        "Usage:\n  $ {} [path to .Civ6Save or .bin file] [name of the output file]\n\n  If the output file ends with .png, a map will be written as an image.\n  Else if the file extension ends with .bin, the decompressed data will be written.",
        args[0]
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 3 {
        let input_filename = &args[1];
        let output_filename = &args[2];
        let input_file_extension = Path::new(input_filename)
            .extension()
            .and_then(OsStr::to_str);
        let output_file_extension = Path::new(output_filename)
            .extension()
            .and_then(OsStr::to_str);

        let mut uncompressed_data = Vec::new();
        if input_file_extension == Some("Civ6Save") {
            let mut base_save_data: Vec<u8> = Vec::new();
            {
                let mut file = File::open(input_filename).unwrap();
                file.read_to_end(&mut base_save_data).unwrap();
            }
            lib::extract_civ6_map_data(&base_save_data, &mut uncompressed_data);
        } else {
            let mut file = File::open(input_filename).unwrap();
            file.read_to_end(&mut uncompressed_data).unwrap();
        }

        if uncompressed_data.len() > 0 {
            // Decompression succeeded
            if output_file_extension == Some("png") {
                // We render the map
                let img = lib::map_render(&uncompressed_data);
                img.save(output_filename).unwrap();
            } else if output_file_extension == Some("bin") {
                let mut output_file = File::create(output_filename).unwrap();
                output_file.write_all(&uncompressed_data).unwrap();
            } else {
                usage(&args);
            }
        } else {
            panic!("Bad decompression! (Decompressed data is empty)");
        }
    } else {
        usage(&args);
    }
}
