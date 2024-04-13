use std::{collections::HashMap, env::args, thread::sleep, time::Duration};
use device_query::{DeviceQuery, DeviceState, Keycode};
use enigo::{Key, KeyboardControllable, MouseButton, MouseControllable};
use image::{imageops::resize, DynamicImage, GenericImageView, Pixel, Rgb, Rgba};

const SMALL_SLEEP_TIME: Duration = Duration::from_millis(5);
const MEDIUM_SLEEP_TIME: Duration = Duration::from_millis(20);
const DOT_WIDTH_FLOAT: f32 = 5.;
const DOT_WIDTH: i32 = 5;
const COLOR_SPACING: i32 = 24;
const BACKGROUND: i32 = -1;

fn main() {
    let args: Vec<String> = args().collect();
    let image_path = args.get(1).expect("Specify image path");
    let img = image::open(image_path).expect("Could not open image");
    let colors = [*Rgb::from_slice(&[0,0,0]), *Rgb::from_slice(&[127,127,127]), *Rgb::from_slice(&[136,0,21]),
                                  *Rgb::from_slice(&[237,28,36]), *Rgb::from_slice(&[255,127,39]), *Rgb::from_slice(&[255,242,0]),
                                  *Rgb::from_slice(&[34,177,76]), *Rgb::from_slice(&[0,162,232]), *Rgb::from_slice(&[63,72,204]),
                                  *Rgb::from_slice(&[163,73,164]), *Rgb::from_slice(&[255,255,255]), *Rgb::from_slice(&[195,195,195]),
                                  *Rgb::from_slice(&[185,122,87]), *Rgb::from_slice(&[255,174,201]), *Rgb::from_slice(&[255,201,14]),
                                  *Rgb::from_slice(&[239,228,176]), *Rgb::from_slice(&[181,230,29]), *Rgb::from_slice(&[153,217,234]),
                                  *Rgb::from_slice(&[112,146,190]), *Rgb::from_slice(&[200,191,231])];
    let mut artist = Artist::new(img, colors);
    let tolerance =  match args.get(2) {
        None => 5.,
        Some(val) => val.parse().expect("Could not parse tolerance"),
    };
    artist.paint(tolerance);
}

enum PaintInstruction {
    Line(i32, i32, i32, i32),
    Color(i32),
    ColorPrecise(Rgb<u8>),
    SetMaxSize,
    SelectBrush,
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
    custom_colors: Vec<Rgb<u8>>,
}

impl Artist {
    fn new(img: DynamicImage, colors: [Rgb<u8>; 20]) -> Self {
        let mut enigo = enigo::Enigo::new();
        let state = device_query::DeviceState::new();
        wait_for_keyup(Keycode::LControl, &state);
        let (mut left, mut top) = enigo.mouse_location();
        wait_for_keyup(Keycode::LControl, &state);
        let (mut right, mut bottom) = enigo.mouse_location();
        wait_for_keyup(Keycode::LControl, &state);
        let (black_x, black_y) = enigo.mouse_location();
        shortcut(&[Key::Control, Key::A], &mut enigo);
        enigo.key_click(Key::Delete);
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
        Self { enigo, img: img.into(), left, top, black_x, black_y, width: horizontal_dots, height: vertical_dots, colors, selected_color: 0, canvas_selected: false, custom_colors: vec![] }
    }

    fn paint_preprocess(&mut self, tolerance: f32) -> (Vec<PaintInstruction>, Vec<Rgb<u8>>, Rgb<u8>) {
        let mut instructions = vec![];
        let mut init_colors = vec![];
        let mut total_colors: HashMap<Rgb<u8>, i32> = HashMap::new();
        for (_, _, color) in self.img.pixels() {
            let blended = blend_with_white(color);
            let mut best_match = None;
            let mut best_match_value = f32::INFINITY;
            for (color, _) in total_colors.iter() {
                let diff = color_difference(blended, *color);
                if diff < best_match_value {
                    best_match = Some(color.clone());
                    best_match_value = diff;
                }
            }
            if best_match_value > tolerance || best_match == None {
                total_colors.insert(blended, 1);
            } else {
                *total_colors.get_mut(&best_match.unwrap()).unwrap() += 1;
            }
        }
        let background = *total_colors.iter().max_by_key(|(_, used)| **used).unwrap().0;
        let background_index = self.colors.iter().enumerate().find(|(_, x)| x == &&background).map(|(i, _)| i as i32).unwrap_or(BACKGROUND);
        for x in 0..self.width {
            let mut line_length = 0;
            let mut line_color = if let Some(color) = self.get_color(x, 0) {
                if color == background {
                    background_index
                } else {
                    let (best_match, diff) = self.get_best_preset(color, background, background_index);
                    if diff > tolerance {
                        if init_colors.len() != 10 {
                            self.selected_color = 20 + init_colors.len() as i32;
                            instructions.push(PaintInstruction::Color(self.selected_color));
                            init_colors.push(color);
                        } else {
                            instructions.push(PaintInstruction::ColorPrecise(color));
                            self.selected_color = 29;
                        }
                        self.custom_colors.push(color);
                        if self.custom_colors.len() == 11 {
                            self.custom_colors.remove(0);
                        }
                        self.selected_color
                    } else {
                        if self.selected_color != best_match && best_match != background_index {
                            instructions.push(PaintInstruction::Color(best_match));
                            self.selected_color = best_match;
                        }
                        best_match
                    }
                }
            } else {
                background_index
            };
            for y in 0..self.height {
                if let Some(color) = self.get_color(x, y) {
                    let (best_match, diff) = self.get_best_preset(color, background, background_index);
                    if diff > tolerance {
                        if line_color != background_index {
                            instructions.push(PaintInstruction::Line(x, y - line_length, x, y - 1));
                        }
                        if color == background {
                            line_length = 1;
                            line_color = background_index;
                        } else {
                            if init_colors.len() != 10 {
                                self.selected_color = 20 + init_colors.len() as i32;
                                instructions.push(PaintInstruction::Color(self.selected_color));
                                init_colors.push(color);
                            } else {
                                instructions.push(PaintInstruction::ColorPrecise(color));
                                self.selected_color = 29;
                            }
                            self.custom_colors.push(color);
                            if self.custom_colors.len() == 11 {
                                self.custom_colors.remove(0);
                            }
                            line_length = 1;
                            line_color = self.selected_color;
                        }
                    } else {
                        if line_color == best_match {
                            line_length += 1;
                        } else {
                            if line_color != background_index {
                                instructions.push(PaintInstruction::Line(x, y - line_length, x, y - 1));
                            }
                            line_length = 1;
                            line_color = best_match;
                            if self.selected_color != best_match && line_color != background_index {
                                instructions.push(PaintInstruction::Color(best_match));
                                self.selected_color = best_match;
                            }
                        }
                    }
                } else if line_color != background_index {
                    instructions.push(PaintInstruction::Line(x, y - line_length, x, y - 1));
                    line_length = 1;
                    line_color = background_index;
                } else {
                    line_length += 1;
                }
            }
            if line_color != background_index {
                instructions.push(PaintInstruction::Line(x, self.height - line_length, x, self.height - 1));
            }
        }
        let mut final_instructions = vec![
            PaintInstruction::SelectBrush,
            PaintInstruction::SetMaxSize,
        ];
        final_instructions.append(&mut instructions);
        (final_instructions, init_colors, background)
    }

    fn paint_from_preprocess(&mut self, instructions: Vec<PaintInstruction>, init_colors: Vec<Rgb<u8>>, background: Rgb<u8>) {
        self.select_square();
        self.set_max_brush_size();
        self.select_color_precise(background, false);
        self.select_color_precise(background, true);
        self.draw_square(0, 0, self.width - 1, self.height - 1);
        for color in &init_colors {
            self.create_color(*color);
        }
        for i in 0..10 - init_colors.len() {
            self.create_color(self.colors[i]);
        }
        self.click(self.left, self.top);
        for instruction in instructions {
            match instruction {
                PaintInstruction::Line(start_x, start_y, end_x, end_y) => self.draw_line(start_x, start_y, end_x, end_y),
                PaintInstruction::Color(index) => self.select_color(index),
                PaintInstruction::ColorPrecise(color) => self.select_color_precise(color, false),
                PaintInstruction::SelectBrush => self.select_brush(),
                PaintInstruction::SetMaxSize => self.set_max_brush_size(),
            }
        }
    }

    fn paint(&mut self, tolerance: f32) {
        let (instructions, init_colors, background) = self.paint_preprocess(tolerance);
        self.paint_from_preprocess(instructions, init_colors, background);
    }

    fn click(&mut self, x: i32, y: i32) {
        self.enigo.mouse_move_to(x, y);
        self.enigo.mouse_click(MouseButton::Left);
        sleep(SMALL_SLEEP_TIME);
        if self.enigo.mouse_location() != (x, y) {
            panic!("Movement detected after clicking");
        }
    }

    fn drag(&mut self, start_x: i32, start_y: i32, end_x: i32, end_y: i32) {
        self.enigo.mouse_move_to(start_x, start_y);
        self.enigo.mouse_down(MouseButton::Left);
        self.enigo.mouse_move_to(end_x, end_y);
        self.enigo.mouse_up(MouseButton::Left);
        sleep(SMALL_SLEEP_TIME);
        if self.enigo.mouse_location() != (end_x, end_y) {
            panic!("Movement detected after dragging");
        }
    }

    fn draw_line(&mut self, start_x: i32, start_y: i32, end_x: i32, end_y: i32) {
        let (start_x, start_y) = (start_x * DOT_WIDTH, start_y * DOT_WIDTH);
        let (end_x, end_y) = (end_x * DOT_WIDTH, end_y * DOT_WIDTH);
        if !self.canvas_selected {
            self.click(self.left + start_x, self.top + start_y);
            self.canvas_selected = true;
        }
        self.drag(self.left + start_x, self.top + start_y, self.left + end_x, self.top + end_y);
    }

    fn draw_square(&mut self, start_x: i32, start_y: i32, end_x: i32, end_y: i32) {
        let (start_x, start_y) = (start_x * DOT_WIDTH + self.left + 2, start_y * DOT_WIDTH + self.top + 2);
        let (end_x, end_y) = (end_x * DOT_WIDTH + self.left, end_y * DOT_WIDTH + self.top);
        if !self.canvas_selected {
            self.click(self.left + start_x, self.top + start_y);
            self.canvas_selected = true;
        }
        self.drag(start_x, start_y, end_x, end_y);
        self.click(start_x, start_y - 20);
    }

    fn get_color(&self, x: i32, y: i32) -> Option<Rgb<u8>> {
        let color: Rgba<u8> = self.img.get_pixel(x as u32, y as u32);
        if color.0[3] == 0 {
            None
        } else if color.0[3] != 255 {
            Some(blend_with_white(color))
        } else {
            Some(color.to_rgb())
        }
    }

    fn get_best_preset(&self, color: Rgb<u8>, background: Rgb<u8>, background_index: i32) -> (i32, f32) {
        let mut best_match: i32 = 0;
        let mut best_match_value = f32::INFINITY;
        for (i, preset) in self.colors.iter().enumerate() {
            let diff = color_difference(color, *preset);
            if diff < best_match_value {
                best_match = i as i32;
                best_match_value = diff;
            }
        }
        for (i, preset) in self.custom_colors.iter().enumerate() {
            let diff = color_difference(color, *preset);
            if diff < best_match_value {
                best_match = i as i32 + 20;
                best_match_value = diff;
            }
        }
        let diff = color_difference(color, background);
        if diff < best_match_value {
            best_match = background_index;
            best_match_value = diff;
        }
        (best_match, best_match_value)
    }

    fn select_color(&mut self, color_index: i32) {
        let column = color_index % 10;
        let row = (color_index - column) / 10;
        let x = column * COLOR_SPACING + self.black_x;
        let y = row * COLOR_SPACING + self.black_y;
        self.click(x, y);
        self.selected_color = color_index;
        self.canvas_selected = false;
    }

    fn select_color_precise(&mut self, color: Rgb<u8>, secondary: bool) {
        if secondary {
            alt_sequence(&[Key::Num2], &mut self.enigo);
            self.create_color(color);
            alt_sequence(&[Key::Num1], &mut self.enigo);
        } else {
            self.create_color(color);
        }
    }

    fn create_color(&mut self, color: Rgb<u8>) {
        alt_sequence(&[Key::E, Key::C], &mut self.enigo);
        for _ in 0..4 {
            self.enigo.key_click(Key::Tab);
        }
        shortcut(&[Key::Control, Key::A], &mut self.enigo);
        self.enigo.key_sequence(&format!("#{:02X?}{:02X?}{:02X?}", color.0[0], color.0[1], color.0[2]));
        for _ in 0..8 {
            self.enigo.key_click(Key::Tab);
        }
        self.enigo.key_click(Key::Return);
        self.canvas_selected = false;
        sleep(MEDIUM_SLEEP_TIME);
    }

    fn set_max_brush_size(&mut self) {
        alt_sequence(&[Key::S, Key::Z], &mut self.enigo);
        self.enigo.key_click(Key::UpArrow);
        self.enigo.key_click(Key::Return);
    }

    fn select_brush(&mut self) {
        alt_sequence(&[Key::B], &mut self.enigo);
    }

    fn select_square(&mut self) {
        for _ in 0..22 {
            self.enigo.key_click(Key::Tab);
        }
        for _ in 0..22 {
            self.enigo.key_click(Key::RightArrow);
        }
        for _ in 0..2 {
            self.enigo.key_click(Key::UpArrow);
        }
        for _ in 0..3 {
            self.enigo.key_click(Key::LeftArrow);
        }
        self.enigo.key_click(Key::Return);
        for _ in 0..2 {
            self.enigo.key_click(Key::Tab);
        }
        self.enigo.key_click(Key::Return);
        self.enigo.key_click(Key::DownArrow);
        self.enigo.key_click(Key::Return);
        self.click(self.left, self.top);
    }
}

fn color_difference(color_1: Rgb<u8>, color_2: Rgb<u8>) -> f32 {
    ((color_1.0[0] as f32 - color_2.0[0] as f32).abs() + (color_1.0[1] as f32 - color_2.0[1] as f32).abs() + (color_1.0[2] as f32 - color_2.0[2] as f32).abs()).sqrt()
}

fn blend_with_white(color: Rgba<u8>) -> Rgb<u8> {
    let alpha_ratio = color.0[3] as f32 / 255.;
    *Rgb::from_slice(&[
        (color.0[0] as f32 * alpha_ratio + 255. - 255. * alpha_ratio).floor() as u8,
        (color.0[1] as f32 * alpha_ratio + 255. - 255. * alpha_ratio).floor() as u8,
        (color.0[2] as f32 * alpha_ratio + 255. - 255. * alpha_ratio).floor() as u8,
    ])
}

fn shortcut(keys: &[Key], enigo: &mut enigo::Enigo) {
    for key in keys {
        enigo.key_down(*key);
    }
    for key in keys {
        enigo.key_up(*key);
    }
}

fn alt_sequence(keys: &[Key], enigo: &mut enigo::Enigo) {
    enigo.key_click(Key::Alt);
    for key in keys {
        enigo.key_click(*key);
    }
}

fn wait_for_keyup(key: Keycode, state: &DeviceState) {
    let mut pressed = false;
    loop {
        if state.get_keys().iter().any(|x| x == &key) {
            pressed = true;
        } else if pressed == true {
            return;
        }
    }
}