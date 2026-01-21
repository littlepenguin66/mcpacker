pub use console::{Emoji, style};

pub static LOOKING_GLASS: Emoji<'_, '_> = Emoji("", "");
pub static SPARKLE: Emoji<'_, '_> = Emoji("", "");

/// Print logo
pub fn print_logo() {
    println!(
        "{}",
        style(
            r#"
===================================================================
███╗   ███╗ ██████╗██████╗  █████╗  ██████╗██╗  ██╗███████╗██████╗
████╗ ████║██╔════╝██╔══██╗██╔══██╗██╔════╝██║ ██╔╝██╔════╝██╔══██╗
██╔████╔██║██║     ██████╔╝███████║██║     █████╔╝ █████╗  ██████╔╝
██║╚██╔╝██║██║     ██╔═══╝ ██╔══██║██║     ██╔═██╗ ██╔══╝  ██╔══██╗
██║ ╚═╝ ██║╚██████╗██║     ██║  ██║╚██████╗██║  ██╗███████╗██║  ██║
╚═╝     ╚═╝ ╚═════╝╚═╝     ╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝
===================================================================
"#
        )
        .bold()
        .cyan()
    );
}

/// Print step message
pub fn print_step(msg: &str) {
    println!("{} {}", style("==>").bold().blue(), style(msg).bold());
}

/// Print header
pub fn print_header(msg: &str) {
    println!("{}", style(msg).bold().underlined());
    println!();
}

/// Print success message
pub fn print_success(msg: &str) {
    println!("{} {}", style("SUCCESS:").green().bold(), msg);
}

/// Print info
pub fn print_info(label: &str, value: &str) {
    println!("  {}: {}", style(label).dim(), style(value).cyan());
}

/// Print warning
pub fn print_warn(msg: &str) {
    println!("{} {}", style("WARNING:").yellow().bold(), msg);
}

/// Print error
#[allow(dead_code)]
pub fn print_error(msg: &str) {
    eprintln!("{} {}", style("ERROR:").red().bold(), msg);
}
