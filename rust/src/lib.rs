#[cfg(test)]
mod test;

mod iterators;

mod pcube;
pub use pcube::{Compression, PolyCubeFile};

mod polycube;
pub use polycube::PolyCube;

pub fn make_bar(len: u64) -> indicatif::ProgressBar {
    use indicatif::{ProgressBar, ProgressStyle};

    let bar = ProgressBar::new(len);

    let pos_width = format!("{len}").len();

    let template =
        format!("[{{elapsed_precise}}] {{bar:40.cyan/blue}} {{pos:>{pos_width}}}/{{len}} {{msg}}");

    bar.set_style(
        ProgressStyle::with_template(&template)
            .unwrap()
            .progress_chars("#>-"),
    );
    bar
}
