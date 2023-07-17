#[cfg(test)]
mod test;

mod pcube_file;
pub use pcube_file::{Compression, PolyCubeFile};

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
