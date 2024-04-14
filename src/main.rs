use std::{fs::File, io::BufReader};
use clap::Parser;
use image::{codecs::gif::{GifDecoder, GifEncoder}, AnimationDecoder};
use crate::paint::{Artist, GifArtist};

mod paint;

/// A program to draw stuff in Microsoft Paint
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to image or gif to paint
    image_path: String,

    /// How similar colors can be
    #[arg(short, long, default_value_t = 5.)]
    tolerance: f32,

    /// How many colors the program can use
    #[arg(short, long, default_value_t = usize::MAX)]
    limit: usize,
}

fn main() {
    let args = Args::parse();
    if args.image_path.ends_with(".gif") {
        let file_in = BufReader::new(File::open(args.image_path).unwrap());
        let decoder = GifDecoder::new(file_in).unwrap();
        let frames = decoder.into_frames().collect_frames().expect("Error decoding gif");
        let mut artist = GifArtist::new(frames, args.tolerance, args.limit);
        let new_frames = artist.paint();
        let file_out = File::create("out.gif").unwrap();
        let mut encoder = GifEncoder::new(file_out);
        encoder.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
        encoder.encode_frames(new_frames.into_iter()).unwrap();
    } else {
        let img = image::open(args.image_path).expect("Could not open image");
        let mut artist = Artist::new(img.into(), args.tolerance, args.limit);
        artist.paint();
        artist.screenshot().save("out.png").unwrap();
    }
}