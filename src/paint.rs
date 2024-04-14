use std::{collections::HashMap, thread::sleep, time::Duration};
use device_query::{DeviceQuery, DeviceState, Keycode};
use enigo::{Key, KeyboardControllable, MouseButton, MouseControllable};
use image::{imageops::{crop_imm, resize}, Delay, DynamicImage, Frame, GenericImageView, ImageBuffer, Pixel, Rgb, Rgba};
use xcap::Monitor;

const SMALL_SLEEP_TIME: Duration = Duration::from_millis(5);
const MEDIUM_SLEEP_TIME: Duration = Duration::from_millis(20);
const LONG_SLEEP_TIME: Duration = Duration::from_millis(200);
const DOT_WIDTH_FLOAT: f32 = 5.;
const DOT_WIDTH: i32 = 5;
const COLOR_SPACING: i32 = 24;

pub struct GifArtist {
    artist: Artist,
    gif: Vec<Frame>,
}

impl GifArtist {
    pub fn new(gif: Vec<Frame>, tolerance: f32, color_limit: usize) -> Self {
        let artist = Artist::new(gif[0].buffer().clone().into(), tolerance, color_limit);
        Self { artist, gif }
    }

    pub fn paint(&mut self) -> Vec<Frame> {
        let mut new_frames = vec![];
        let frame = &self.gif[0];
        new_frames.push(self.paint_frame(frame.left(), frame.top(), frame.delay()));
        for frame_index in 1..self.gif.len() {
            let frame = &self.gif[frame_index];
            self.artist.new_image(frame.buffer().clone());
            new_frames.push(self.paint_frame(frame.left(), frame.top(), frame.delay()));
        }
        new_frames
    }

    fn paint_frame(&mut self, left: u32, top: u32, delay: Delay) -> Frame {
        self.artist.paint();
        let img = self.artist.screenshot();
        Frame::from_parts(img, left, top, delay)
    }
}

enum PaintInstruction {
    Line(i32, i32, i32, i32),
    Color(i32),
    ColorPrecise(Rgb<u8>),
    SetMaxSize,
    SelectBrush,
}

pub struct Artist {
    enigo: enigo::Enigo,
    img: DynamicImage,
    left: i32,
    top: i32,
    black_x: i32,
    black_y: i32,
    width: i32,
    height: i32,
    colors: [Rgb<u8>; 20],
    canvas_selected: bool,
    tolerance: f32,
    color_limit: usize,
}

impl Artist {
    pub fn new(img: ImageBuffer<Rgba<u8>, Vec<u8>>, tolerance: f32, color_limit: usize) -> Self {
        let colors = [*Rgb::from_slice(&[0,0,0]), *Rgb::from_slice(&[127,127,127]), *Rgb::from_slice(&[136,0,21]),
                                     *Rgb::from_slice(&[237,28,36]), *Rgb::from_slice(&[255,127,39]), *Rgb::from_slice(&[255,242,0]),
                                     *Rgb::from_slice(&[34,177,76]), *Rgb::from_slice(&[0,162,232]), *Rgb::from_slice(&[63,72,204]),
                                     *Rgb::from_slice(&[163,73,164]), *Rgb::from_slice(&[255,255,255]), *Rgb::from_slice(&[195,195,195]),
                                     *Rgb::from_slice(&[185,122,87]), *Rgb::from_slice(&[255,174,201]), *Rgb::from_slice(&[255,201,14]),
                                     *Rgb::from_slice(&[239,228,176]), *Rgb::from_slice(&[181,230,29]), *Rgb::from_slice(&[153,217,234]),
                                     *Rgb::from_slice(&[112,146,190]), *Rgb::from_slice(&[200,191,231])];
        let enigo = enigo::Enigo::new();
        let state = device_query::DeviceState::new();
        wait_for_keyup(Keycode::LControl, &state);
        let (mut left, mut top) = enigo.mouse_location();
        wait_for_keyup(Keycode::LControl, &state);
        let (mut right, mut bottom) = enigo.mouse_location();
        wait_for_keyup(Keycode::LControl, &state);
        let (black_x, black_y) = enigo.mouse_location();
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
        Self { enigo, img: img.into(), left, top, black_x, black_y, width: horizontal_dots, height: vertical_dots, colors, canvas_selected: false, tolerance, color_limit }
    }

    fn new_image(&mut self, img: ImageBuffer<Rgba<u8>, Vec<u8>>) {
        let img = resize(&img, self.width as u32, self.height as u32, image::imageops::FilterType::Gaussian);
        self.img = img.into();
    }

    fn paint_preprocess(&mut self) -> (Vec<PaintInstruction>, Vec<Rgb<u8>>, Rgb<u8>) {
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
            if best_match_value > self.tolerance || best_match == None {
                total_colors.insert(blended, 1);
            } else {
                *total_colors.get_mut(&best_match.unwrap()).unwrap() += 1;
            }
        }
        let background = *total_colors.iter().max_by_key(|(_, used)| **used).unwrap().0;
        let mut draw_batches: [Vec<(i32, i32)>; 20] = Default::default();
        let mut custom_draw_batches: Vec<(Rgb<u8>, Vec<(i32, i32)>)> = vec![];
        let mut pixels: Vec<(u32, u32, Rgb<u8>)> = self.img.pixels().map(|(x, y, color)| (x, y, blend_with_white(color))).filter(|(_, _, color)| {
            let diff = color_difference(*color, background);
            diff > self.tolerance
        }).collect();
        if self.color_limit == 0 {
            for (x, y, color) in pixels.iter().rev() {
                let mut best_match = 0;
                let mut best_match_value = f32::INFINITY;
                for (i, preset) in self.colors.iter().enumerate() {
                    let diff = color_difference(*color, *preset);
                    if diff < best_match_value {
                        best_match = i;
                        best_match_value = diff;
                    }
                }
                draw_batches[best_match].push((*x as i32, *y as i32));
            }
        } else  {
            for (i, (x, y, color)) in pixels.clone().iter().enumerate().rev() {
                let mut best_match = 0;
                let mut best_match_value = f32::INFINITY;
                for (i, preset) in self.colors.iter().enumerate() {
                    let diff = color_difference(*color, *preset);
                    if diff < best_match_value {
                        best_match = i;
                        best_match_value = diff;
                    }
                }
                if best_match_value <= self.tolerance {
                    pixels.remove(i);
                    draw_batches[best_match].push((*x as i32, *y as i32));
                }
            }
            while pixels.len() != 0 && custom_draw_batches.len() < self.color_limit - 1 {
                let mut total_colors: HashMap<Rgb<u8>, i32> = HashMap::new();
                for (_, _, color) in &pixels {
                    let mut best_match = None;
                    let mut best_match_value = f32::INFINITY;
                    for (color_2, _) in total_colors.iter() {
                        let diff = color_difference(*color, *color_2);
                        if diff < best_match_value {
                            best_match = Some(color_2.clone());
                            best_match_value = diff;
                        }
                    }
                    if best_match_value > self.tolerance || best_match == None {
                        total_colors.insert(*color, 1);
                    } else {
                        *total_colors.get_mut(&best_match.unwrap()).unwrap() += 1;
                    }
                }
                let most_common = total_colors.iter().max_by_key(|(_, used)| **used).unwrap().0;
                custom_draw_batches.push((*most_common, vec![]));
                let index = custom_draw_batches.len() - 1;
                for (i, (x, y, color)) in pixels.clone().iter().enumerate().rev() {
                    let diff = color_difference(*color, *most_common);
                    if diff <= self.tolerance {
                        pixels.remove(i);
                        custom_draw_batches[index].1.push((*x as i32, *y as i32));
                    }
                }
            }
            if custom_draw_batches.len() == self.color_limit - 1 && pixels.len() != 0 {
                let mut average_r = 0.;
                let mut average_g = 0.;
                let mut average_b = 0.;
                for (_, _, color) in &pixels {
                    average_r += color.0[0] as f32;
                    average_g += color.0[1] as f32;
                    average_b += color.0[2] as f32;
                }
                let len = pixels.len() as f32;
                average_r /= len;
                average_g /= len;
                average_b /= len;
                custom_draw_batches.push((*Rgb::from_slice(&[average_r as u8, average_g as u8, average_b as u8]), pixels.iter().rev().map(|(x, y, _)| (*x as i32, *y as i32)).collect()));
            }
        }
        let mut final_instructions = vec![
            PaintInstruction::SelectBrush,
            PaintInstruction::SetMaxSize,
        ];
        for (i, batch) in draw_batches.iter().enumerate() {
            if batch.len() != 0 {
                final_instructions.push(PaintInstruction::Color(i as i32));
                let mut line_length = 0;
                let mut iter = batch.iter();
                let (mut line_x, mut line_y) = iter.next().unwrap();
                for (x, y) in iter {
                    if *y == line_y && *x == line_x - 1 {
                        line_length += 1;
                    } else {
                        final_instructions.push(PaintInstruction::Line(line_x + line_length, line_y, line_x, line_y));
                        line_length = 0;
                    }
                    line_x = *x;
                    line_y = *y;
                }
                final_instructions.push(PaintInstruction::Line(line_x + line_length, line_y, line_x, line_y));
            }
        }
        for (color, batch) in custom_draw_batches {
            if batch.len() != 0 {
                if init_colors.len() != 10 {
                    let color_index = 20 + init_colors.len() as i32;
                    final_instructions.push(PaintInstruction::Color(color_index));
                    init_colors.push(color);
                } else {
                    final_instructions.push(PaintInstruction::ColorPrecise(color));
                }
                let mut line_length = 0;
                let mut iter = batch.iter();
                let (mut line_x, mut line_y) = iter.next().unwrap();
                for (x, y) in iter {
                    if *y == line_y && *x == line_x - 1 {
                        line_length += 1;
                    } else {
                        final_instructions.push(PaintInstruction::Line(line_x + line_length, line_y, line_x, line_y));
                        line_length = 0;
                    }
                    line_x = *x;
                    line_y = *y;
                }
                final_instructions.push(PaintInstruction::Line(line_x + line_length, line_y, line_x, line_y));
            }
        }
        final_instructions.append(&mut instructions);
        (final_instructions, init_colors, background)
    }

    fn paint_from_preprocess(&mut self, instructions: Vec<PaintInstruction>, init_colors: Vec<Rgb<u8>>, background: Rgb<u8>) {
        self.click(self.left, self.top);
        shortcut(&[Key::Control, Key::A], &mut self.enigo);
        self.enigo.key_click(Key::Delete);
        self.select_square();
        self.set_max_brush_size();
        self.select_color_precise(background, false);
        self.select_color_precise(background, true);
        self.draw_square(0, 0, self.width - 1, self.height - 1);
        if init_colors.len() != 0 {
            for color in &init_colors {
                self.create_color(*color);
            }
            for i in 0..10 - init_colors.len() {
                self.create_color(self.colors[i]);
            }
        }
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

    pub fn paint(&mut self) {
        let (instructions, init_colors, background) = self.paint_preprocess();
        self.paint_from_preprocess(instructions, init_colors, background);
    }

    pub fn screenshot(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        sleep(LONG_SLEEP_TIME);
        let monitor = Monitor::from_point(self.left, self.top).unwrap();
        let img = monitor.capture_image().unwrap();
        crop_imm(&img, self.left as u32, self.top as u32, (self.width * DOT_WIDTH - 1) as u32, (self.height * DOT_WIDTH - 1) as u32).to_image()
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
        self.drag(start_x, start_y, end_x, end_y);
        self.click(start_x, start_y - 20);
    }

    fn select_color(&mut self, color_index: i32) {
        let column = color_index % 10;
        let row = (color_index - column) / 10;
        let x = column * COLOR_SPACING + self.black_x;
        let y = row * COLOR_SPACING + self.black_y;
        self.click(x, y);
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
        sleep(MEDIUM_SLEEP_TIME);
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
        for _ in 0..7 {
            self.enigo.key_click(Key::LeftArrow);
        }
        for _ in 0..2 {
            self.enigo.key_click(Key::UpArrow);
        }
        for _ in 0..3 {
            self.enigo.key_click(Key::RightArrow);
        }
        self.enigo.key_click(Key::Return);
        sleep(MEDIUM_SLEEP_TIME);
        for _ in 0..2 {
            self.enigo.key_click(Key::Tab);
        }
        self.enigo.key_click(Key::Return);
        sleep(MEDIUM_SLEEP_TIME);
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