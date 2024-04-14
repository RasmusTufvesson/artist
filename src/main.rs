use std::{env::args, fs::File, io::BufReader};
use image::{codecs::gif::{GifDecoder, GifEncoder}, AnimationDecoder};
use crate::paint::{ColorLimit, Artist, GifArtist};

mod paint;

fn main() {
    let args: Vec<String> = args().collect();
    let image_path = args.get(1).expect("Specify image path");
    let tolerance =  match args.get(2) {
        None => 5.,
        Some(val) => val.parse().expect("Could not parse tolerance"),
    };
    let color_limit =  match args.get(3) {
        None => ColorLimit::None,
        Some(val) => {
            if val == "primary" {
                ColorLimit::OnlyPrimary
            } else if val == "first" {
                ColorLimit::OnlyFirstCustom
            } else if val == "none" {
                ColorLimit::None
            } else {
                ColorLimit::Custom(val.parse().expect("Could not parse color limit"))
            }
        }
    };
    if image_path.ends_with(".gif") {
        let file_in = BufReader::new(File::open(image_path).unwrap());
        let decoder = GifDecoder::new(file_in).unwrap();
        let frames = decoder.into_frames().collect_frames().expect("Error decoding gif");
        let mut artist = GifArtist::new(frames, tolerance, color_limit);
        let new_frames = artist.paint();
        let file_out = File::create("out.gif").unwrap();
        let mut encoder = GifEncoder::new(file_out);
        encoder.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
        encoder.encode_frames(new_frames.into_iter()).unwrap();
    } else {
        let img = image::open(image_path).expect("Could not open image");
        let mut artist = Artist::new(img.into());
        artist.paint(tolerance, color_limit);
        artist.screenshot().save("out.png").unwrap();
    }
}