use chrono::{Local, Timelike};
use clap::{Parser, ValueEnum};
use image::GenericImageView;
use sdl2::event::Event;
use sdl2::keyboard::{KeyboardUtil, Keycode, Mod};
use sdl2::pixels::{Color, PixelMasks};
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
use sdl2::surface::Surface;
use sdl2::video::{FullscreenType, Window, WindowContext};
use sdl2::TimerSubsystem;
use std::borrow::Borrow;
use std::path::Path;

const FPS: u32 = 60;
const SPRITE_CHAR_WIDTH: u32 = 300 / 2;
const SPRITE_CHAR_HEIGHT: u32 = 380 / 2;
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
const BACKGROUND_COLOR_R: u8 = 24;
const BACKGROUND_COLOR_G: u8 = 24;
const BACKGROUND_COLOR_B: u8 = 24;
const WIGGLE_COUNT: usize = 3;
const WIGGLE_DURATION: f32 = 0.4 / WIGGLE_COUNT as f32;
const COLON_INDEX: usize = 10;
const SCALE_FACTOR: f32 = 0.15;

fn load_png_as_texture<'a>(texture_creator: &'a TextureCreator<WindowContext>) -> Texture<'a> {
    let filepath = "./digits.png";
    let img = match image::open(&Path::new(&filepath)) {
        Ok(img) => img,
        Err(_) => {
            eprintln!("Could not load file {}", filepath);
            std::process::exit(1);
        }
    };

    let (png_width, png_height) = img.dimensions();
    let mut png_data = img.to_rgba8().into_raw();

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

    let digits_texture = texture_creator
        .create_texture_from_surface(surface)
        .unwrap();
    digits_texture
}

fn initial_pen(
    window: &Window,
    pen_x: &mut i32,
    pen_y: &mut i32,
    user_scale: f32,
    fit_scale: &mut f32,
) {
    let (w, h) = window.size();

    let text_aspect_ratio = TEXT_WIDTH as f64 / TEXT_HEIGHT as f64;
    let window_aspect_radio = w as f64 / h as f64;
    *fit_scale = if text_aspect_ratio > window_aspect_radio {
        w as f32 / TEXT_WIDTH as f32
    } else {
        h as f32 / TEXT_HEIGHT as f32
    };

    let effective_digit_width = (CHAR_WIDTH as f32 * user_scale * *fit_scale).floor() as i32;
    let effective_digit_height = (CHAR_HEIGHT as f32 * user_scale * *fit_scale).floor() as i32;
    *pen_x = w as i32 / 2 - effective_digit_width * CHARS_COUNT as i32 / 2;
    *pen_y = h as i32 / 2 - effective_digit_height / 2;
}

fn render_digit_at(
    renderer: &mut WindowCanvas,
    texture: &Texture,
    digit_index: usize,
    wiggle_index: usize,
    pen_x: &mut i32,
    pen_y: &mut i32,
    user_scale: f32,
    fit_scale: f32,
) {
    let effective_digit_width = (CHAR_WIDTH as f32 * user_scale * fit_scale).floor() as i32;
    let effective_digit_height = (CHAR_HEIGHT as f32 * user_scale * fit_scale).floor() as i32;
    let src_rect = Rect::new(
        (digit_index * CHAR_WIDTH as usize) as i32,
        (wiggle_index * CHAR_HEIGHT as usize) as i32,
        SPRITE_CHAR_WIDTH,
        SPRITE_CHAR_HEIGHT,
    );

    let dst_rect = Rect::new(
        *pen_x,
        *pen_y,
        effective_digit_width as u32,
        effective_digit_height as u32,
    );

    renderer.copy(texture, src_rect, dst_rect).unwrap();
    *pen_x += effective_digit_width;
}

struct FpsDeltaTime {
    pub frame_delay: u32, // Frame delay in milliseconds
    pub dt: f32,          // Delta time in seconds
    pub last_time: u64,
    timer_subsystem: TimerSubsystem,
}

impl FpsDeltaTime {
    pub fn new(fps_cap: u32, timer_subsystem: TimerSubsystem) -> Self {
        Self {
            last_time: timer_subsystem.performance_counter(),
            timer_subsystem,
            frame_delay: 1000 / fps_cap,
            dt: 0.0,
        }
    }

    pub fn frame_start(&mut self) {
        let now = self.timer_subsystem.performance_counter();
        let elapsed = now - self.last_time;
        self.dt = elapsed as f32 / self.timer_subsystem.performance_frequency() as f32;
        self.last_time = now;
    }

    pub fn frame_end(&self) {
        let now = self.timer_subsystem.performance_counter();
        let elapsed = now - self.last_time;
        let cap_frame_end = (((elapsed as f32) * 1000.0)
            / (self.timer_subsystem.performance_frequency() as f32))
            as u32;

        if cap_frame_end < self.frame_delay {
            self.timer_subsystem.delay(self.frame_delay - cap_frame_end);
        }
    }
}

#[derive(Debug, ValueEnum, Clone)]
enum Mode {
    Ascending,
    Countdown,
    Clock,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Start pause timer
    #[arg(short = 'p', long, default_value_t = false)]
    pause: bool,

    /// Mode
    #[arg(short = 'm', value_enum, long, default_value_t = Mode::Ascending)]
    mode: Mode,

    /// In Countdown mode, exit when done.
    #[arg(short = 'e', long, default_value_t = false)]
    exit_after_countdown: bool,

    /// Countdown seconds
    #[arg(required_if_eq("mode", "countdown"))]
    seconds: Option<u32>,
}

pub fn main() {
    let args = Cli::parse();
    let mut displayed_time = args.seconds.unwrap_or(0) as f32;
    let mut paused = args.pause;

    let sdl_context = sdl2::init().unwrap();
    let timer_subsystem = sdl_context.timer().unwrap();
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
    let mut digits_texture = load_png_as_texture(&texture_creator);

    if paused {
        digits_texture.set_color_mod(PAUSE_COLOR_R, PAUSE_COLOR_G, PAUSE_COLOR_B);
    } else {
        digits_texture.set_color_mod(MAIN_COLOR_R, MAIN_COLOR_G, MAIN_COLOR_B);
    }

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut user_scale = 1.0;
    let mut wiggle_index = 0;
    let mut fps_dt = FpsDeltaTime::new(FPS, timer_subsystem);
    let mut wiggle_cooldown = WIGGLE_DURATION;

    'running: loop {
        fps_dt.frame_start();
        // input begin
        let keyboard_state = event_pump.keyboard_state();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,

                Event::MouseWheel { y, .. } => {
                    if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::LCtrl)
                        || keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::RCtrl)
                    {
                        if y > 0 {
                            user_scale += SCALE_FACTOR * user_scale;
                        } else if y < 0 {
                            user_scale -= SCALE_FACTOR * user_scale;
                        }
                    }
                }
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::KpPlus | Keycode::Equals => {
                        user_scale += SCALE_FACTOR * user_scale;
                    }
                    Keycode::KpMinus | Keycode::Minus => {
                        user_scale -= SCALE_FACTOR * user_scale;
                    }
                    Keycode::Kp0 | Keycode::Num0 => {
                        user_scale = 1.0;
                    }
                    Keycode::F5 => {
                        displayed_time = args.seconds.unwrap_or(0) as f32;
                        paused = args.pause;

                        if paused {
                            digits_texture.set_color_mod(
                                PAUSE_COLOR_R,
                                PAUSE_COLOR_G,
                                PAUSE_COLOR_B,
                            );
                        } else {
                            digits_texture.set_color_mod(MAIN_COLOR_R, MAIN_COLOR_G, MAIN_COLOR_B);
                        }
                    }
                    Keycode::F11 => {
                        let window_flags = canvas.window().window_flags();
                        let window_type = FullscreenType::from_window_flags(window_flags);
                        if window_type == FullscreenType::Desktop {
                            canvas
                                .window_mut()
                                .set_fullscreen(FullscreenType::Off)
                                .unwrap();
                        } else {
                            canvas
                                .window_mut()
                                .set_fullscreen(FullscreenType::Desktop)
                                .unwrap();
                        }
                    }
                    Keycode::SPACE => {
                        paused = !paused;
                        if paused {
                            digits_texture.set_color_mod(
                                PAUSE_COLOR_R,
                                PAUSE_COLOR_G,
                                PAUSE_COLOR_B,
                            );
                        } else {
                            digits_texture.set_color_mod(MAIN_COLOR_R, MAIN_COLOR_G, MAIN_COLOR_B);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        // input end

        // render begin
        canvas.set_draw_color(Color {
            r: BACKGROUND_COLOR_R,
            g: BACKGROUND_COLOR_G,
            b: BACKGROUND_COLOR_B,
            a: 255,
        });
        canvas.clear();
        {
            let mut pen_x = 0;
            let mut pen_y = 0;
            let mut fit_scale = 1.0;

            initial_pen(
                canvas.window(),
                &mut pen_x,
                &mut pen_y,
                user_scale,
                &mut fit_scale,
            );

            let hours = (displayed_time / 60.0 / 60.0) as usize;
            render_digit_at(
                &mut canvas,
                &digits_texture,
                hours / 10,
                wiggle_index % WIGGLE_COUNT,
                &mut pen_x,
                &mut pen_y,
                user_scale,
                fit_scale,
            );
            render_digit_at(
                &mut canvas,
                &digits_texture,
                hours % 10,
                (wiggle_index + 1) % WIGGLE_COUNT,
                &mut pen_x,
                &mut pen_y,
                user_scale,
                fit_scale,
            );
            render_digit_at(
                &mut canvas,
                &digits_texture,
                COLON_INDEX,
                (wiggle_index + 1) % WIGGLE_COUNT,
                &mut pen_x,
                &mut pen_y,
                user_scale,
                fit_scale,
            );

            let minutes = (displayed_time / 60.0 % 60.0) as usize;
            render_digit_at(
                &mut canvas,
                &digits_texture,
                minutes / 10,
                (wiggle_index + 2) % WIGGLE_COUNT,
                &mut pen_x,
                &mut pen_y,
                user_scale,
                fit_scale,
            );
            render_digit_at(
                &mut canvas,
                &digits_texture,
                minutes % 10,
                (wiggle_index + 3) % WIGGLE_COUNT,
                &mut pen_x,
                &mut pen_y,
                user_scale,
                fit_scale,
            );
            render_digit_at(
                &mut canvas,
                &digits_texture,
                COLON_INDEX,
                (wiggle_index + 1) % WIGGLE_COUNT,
                &mut pen_x,
                &mut pen_y,
                user_scale,
                fit_scale,
            );

            let seconds = (displayed_time % 60.0) as usize;
            render_digit_at(
                &mut canvas,
                &digits_texture,
                seconds / 10,
                (wiggle_index + 4) % WIGGLE_COUNT,
                &mut pen_x,
                &mut pen_y,
                user_scale,
                fit_scale,
            );
            render_digit_at(
                &mut canvas,
                &digits_texture,
                seconds % 10,
                (wiggle_index + 5) % WIGGLE_COUNT,
                &mut pen_x,
                &mut pen_y,
                user_scale,
                fit_scale,
            );
            canvas
                .window_mut()
                .set_title(format!("{:02}:{:02}:{:02} - sowon", hours, minutes, seconds).as_str())
                .unwrap();
        }
        canvas.present();
        // render end

        if wiggle_cooldown <= 0.0 {
            wiggle_index += 1;
            wiggle_cooldown = WIGGLE_DURATION;
        }
        wiggle_cooldown -= fps_dt.dt;

        if !paused {
            match args.mode {
                Mode::Ascending => displayed_time += fps_dt.dt,
                Mode::Countdown => {
                    if displayed_time > 1e-6 {
                        displayed_time -= fps_dt.dt;
                    } else {
                        displayed_time = 0.0;
                        if args.exit_after_countdown {
                            std::process::exit(0);
                        }
                    }
                }
                Mode::Clock => {
                    let now = Local::now();
                    let hours = now.hour() as f32;
                    let minutes = now.minute() as f32;
                    let seconds = now.second() as f32;

                    displayed_time = seconds + (minutes * 60.0) + (hours * 60.0 * 60.0);
                }
            }
        }
        fps_dt.frame_end();
    }
}
