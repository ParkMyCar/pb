use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

fn main() {
    let spinner = ProgressBar::new_spinner().with_message("Hello world!");
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")
            .unwrap()
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(&["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"]),
    );
    std::thread::sleep(Duration::from_secs(5));
    spinner.finish();
}
