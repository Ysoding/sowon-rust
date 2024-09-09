use image::GenericImageView;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelMasks};
use sdl2::surface::Surface;
use std::path::Path;
use std::time::Duration;

const CHAR_HEIGHT: u32 = 380 / 2;
const CHAR_WIDTH: u32 = 300 / 2;
const CHARS_COUNT: u32 = 8;
const TEXT_WIDTH: u32 = CHAR_WIDTH * CHARS_COUNT;
const TEXT_HEIGHT: u32 = CHAR_HEIGHT;
const MAIN_COLOR_R: u8 = 220;
const MAIN_COLOR_G: u8 = 220;
const MAIN_COLOR_B: u8 = 220;
const PAUSE_COLOR_R: u8 = 220;
const PAUSE_COLOR_G: u8 = 120;
const PAUSE_COLOR_B: u8 = 120;

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("sowon", TEXT_WIDTH, TEXT_HEIGHT)
        .resizable()
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .target_texture()
        .build()
        .unwrap();

    sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "linear");

    let texture_creator = canvas.texture_creator();

    let filepath = "./digits.png";
    let img = match image::open(&Path::new(&filepath)) {
        Ok(img) => img,
        Err(_) => {
            eprintln!("Could not load file {}", filepath);
            std::process::exit(1);
        }
    };

    let (png_width, png_height) = img.dimensions();
    // let png = img.to_rgba8();
    let mut png_data = img.to_rgba8().into_raw(); // Extract the raw Vec<u8>

    let surface = Surface::from_data_pixelmasks(
        png_data.as_mut_slice(),
        png_width,
        png_height,
        png_width * 4,
        &PixelMasks {
            bpp: 32,
            rmask: 0x000000FF,
            gmask: 0x0000FF00,
            bmask: 0x00FF0000,
            amask: 0xFF000000,
        },
    )
    .unwrap();

    let mut texture = texture_creator
        .create_texture_from_surface(surface)
        .unwrap();

    texture.set_color_mod(MAIN_COLOR_R, MAIN_COLOR_G, MAIN_COLOR_B);

    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0;
    'running: loop {
        i = (i + 1) % 255;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }
        // The rest of the game loop goes here...

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
