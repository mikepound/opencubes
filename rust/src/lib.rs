#[cfg(test)]
mod test;

pub mod pcube;

pub mod naive_polycube;

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
