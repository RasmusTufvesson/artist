use std::{env::args, thread::sleep, time::Duration};
use enigo::{Key, KeyboardControllable, MouseButton, MouseControllable};
use image::{imageops::resize, DynamicImage, GenericImageView, Pixel, Rgb, Rgba};

const SLEEP_TIME: Duration = Duration::from_secs(3);
const SMALL_SLEEP_TIME: Duration = Duration::from_millis(5);
const DOT_WIDTH_FLOAT: f32 = 5.;
const DOT_WIDTH: i32 = 5;
const COLOR_SPACING: i32 = 26;

fn main() {
    let image_path = args().nth(1).expect("Specify image path");
    let img = image::open(image_path).expect("Could not open image");
    let colors = [*Rgb::from_slice(&[0,0,0]), *Rgb::from_slice(&[127,127,127]), *Rgb::from_slice(&[136,0,21]),
                                  *Rgb::from_slice(&[237,28,36]), *Rgb::from_slice(&[255,127,39]), *Rgb::from_slice(&[255,242,0]),
                                  *Rgb::from_slice(&[34,177,76]), *Rgb::from_slice(&[0,162,232]), *Rgb::from_slice(&[63,72,204]),
                                  *Rgb::from_slice(&[163,73,164]), *Rgb::from_slice(&[255,255,255]), *Rgb::from_slice(&[195,195,195]),
                                  *Rgb::from_slice(&[185,122,87]), *Rgb::from_slice(&[255,174,201]), *Rgb::from_slice(&[255,201,14]),
                                  *Rgb::from_slice(&[239,228,176]), *Rgb::from_slice(&[181,230,29]), *Rgb::from_slice(&[153,217,234]),
                                  *Rgb::from_slice(&[112,146,190]), *Rgb::from_slice(&[200,191,231])];
    let mut artist = Artist::new(img, colors);
    artist.paint();
}

struct Artist {
    enigo: enigo::Enigo,
    img: DynamicImage,
    left: i32,
    top: i32,
    black_x: i32,
    black_y: i32,
    width: i32,
    height: i32,
    colors: [Rgb<u8>; 20],
    selected_color: i32,
    canvas_selected: bool,
}

impl Artist {
    fn new(img: DynamicImage, colors: [Rgb<u8>; 20]) -> Self {
        let mut enigo = enigo::Enigo::new();
        sleep(SLEEP_TIME);
        let (mut left, mut top) = enigo.mouse_location();
        enigo.mouse_click(MouseButton::Left);
        sleep(SLEEP_TIME);
        let (mut right, mut bottom) = enigo.mouse_location();
        enigo.mouse_click(MouseButton::Left);
        sleep(SLEEP_TIME);
        let (black_x, black_y) = enigo.mouse_location();
        enigo.key_down(Key::Control);
        enigo.key_down(Key::A);
        enigo.key_up(Key::Control);
        enigo.key_up(Key::A);
        enigo.key_click(Key::Delete);
        enigo.key_click(Key::Alt);
        enigo.key_click(Key::B);
        enigo.key_click(Key::Alt);
        enigo.key_click(Key::S);
        enigo.key_click(Key::Z);
        enigo.key_click(Key::UpArrow);
        enigo.key_click(Key::Return);
        if right < left {
            (right, left) = (left, right);
        }
        if bottom < top {
            (top, bottom) = (bottom, top);
        }
        let painting_width = right - left;
        let painting_height = bottom - top;
        let horizontal_dots = (painting_width as f32 / DOT_WIDTH_FLOAT).ceil() as i32;
        let vertical_dots = (painting_height as f32 / DOT_WIDTH_FLOAT).ceil() as i32;
        let img = resize(&img, horizontal_dots as u32, vertical_dots as u32, image::imageops::FilterType::Gaussian);
        Self { enigo, img: img.into(), left, top, black_x, black_y, width: horizontal_dots, height: vertical_dots, colors, selected_color: 0, canvas_selected: false }
    }

    fn paint(&mut self) {
        for x in 0..self.width {
            let mut line_length = 0;
            let mut line_color = self.get_color(x, 0);
            if line_color != 10 {
                self.select_color(line_color);
            }
            for y in 0..self.height {
                let color = self.get_color(x, y);
                if color == line_color {
                    line_length += 1;
                } else {
                    if line_color != 10 {
                        self.draw_line(x * DOT_WIDTH, (y - line_length) * DOT_WIDTH, x * DOT_WIDTH, (y - 1) * DOT_WIDTH);
                    }
                    line_length = 1;
                    line_color = color;
                    if line_color != 10 {
                        self.select_color(color);
                    }
                }
            }
            if line_color != 10 {
                self.draw_line(x * DOT_WIDTH, (self.height - line_length - 1) * DOT_WIDTH, x * DOT_WIDTH, (self.height - 1) * DOT_WIDTH);
            }
        }
    }

    fn click(&mut self, x: i32, y: i32) {
        sleep(SMALL_SLEEP_TIME);
        self.enigo.mouse_move_to(x, y);
        self.enigo.mouse_click(MouseButton::Left);
    }

    fn drag(&mut self, start_x: i32, start_y: i32, end_x: i32, end_y: i32) {
        sleep(SMALL_SLEEP_TIME);
        self.enigo.mouse_move_to(start_x, start_y);
        self.enigo.mouse_down(MouseButton::Left);
        self.enigo.mouse_move_to(end_x, end_y);
        self.enigo.mouse_up(MouseButton::Left);
    }

    fn draw_line(&mut self, start_x: i32, start_y: i32, end_x: i32, end_y: i32) {
        if !self.canvas_selected {
            self.click(self.left + start_x, self.top + start_y);
            self.canvas_selected = true;
        }
        self.drag(self.left + start_x, self.top + start_y, self.left + end_x, self.top + end_y);
    }

    fn get_color(&self, x: i32, y: i32) -> i32 {
        let color: Rgba<u8> = self.img.get_pixel(x as u32, y as u32);
        if color.0[3] == 0 {
            10
        } else {
            self.get_best_color(color.to_rgb())
        }
    }

    fn get_best_color(&self, color: Rgb<u8>) -> i32 {
        let mut best_match: i32 = 0;
        let mut best_match_value = f32::INFINITY;
        for i in 0..self.colors.len() {
            let diff = color_difference(color, self.colors[i]);
            if diff < best_match_value {
                best_match = i as i32;
                best_match_value = diff;
            }
        }
        best_match
    }

    fn select_color(&mut self, color_index: i32) {
        if self.selected_color != color_index {
            let column = color_index % 10;
            let row = (color_index - column) / 10;
            let x = column * COLOR_SPACING + self.black_x;
            let y = row * COLOR_SPACING + self.black_y;
            self.click(x, y);
            self.selected_color = color_index;
            self.canvas_selected = false;
        }
    }
}

fn color_difference(color_1: Rgb<u8>, color_2: Rgb<u8>) -> f32 {
    ((color_1.0[0] as f32 - color_2.0[0] as f32).abs() + (color_1.0[1] as f32 - color_2.0[1] as f32).abs() + (color_1.0[2] as f32 - color_2.0[2] as f32).abs()).sqrt()
}