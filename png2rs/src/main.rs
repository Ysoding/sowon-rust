use image::GenericImageView;
use std::io::Write;
use std::{env, fs::File, path::Path};

fn shift(args: &mut Vec<String>) -> Option<String> {
    if args.is_empty() {
        None
    } else {
        Some(args.remove(0))
    }
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    shift(&mut args);

    if args.is_empty() {
        eprintln!("Usage: png2rs <filepath.png> <output.rs>");
        std::process::exit(1);
    }

    let filepath = shift(&mut args).unwrap();
    let output_filepath = shift(&mut args).unwrap_or("output.rs".to_string());

    let img = match image::open(&Path::new(&filepath)) {
        Ok(img) => img,
        Err(_) => {
            eprintln!("Could not load file {}", filepath);
            std::process::exit(1);
        }
    };

    let (x, y) = img.dimensions();
    let pixels = img.to_rgba8();
    let mut output_file = match File::create(&output_filepath) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("Could not create file {}", output_filepath);
            std::process::exit(1);
        }
    };

    writeln!(
        output_file,
        "// Generated Rust code containing the image data"
    )
    .unwrap();

    writeln!(output_file, "pub const PNG_WIDTH: u32 = {};", x).unwrap();
    writeln!(output_file, "pub const PNG_HEIGHT: u32 = {};", y).unwrap();
    write!(output_file, "pub const PNG: [u8; {}] = [", x * y * 4).unwrap();

    for (_, pixel_chunk) in pixels.chunks(1).enumerate() {
        // let pixel = u32::from_le_bytes([
        //     pixel_chunk[0],
        //     pixel_chunk[1],
        //     pixel_chunk[2],
        //     pixel_chunk[3],
        // ]);
        write!(output_file, "0x{}, ", pixel_chunk[0]).unwrap();
    }

    writeln!(output_file, "];").unwrap();
    writeln!(output_file, "// End of generated Rust code").unwrap();
}
