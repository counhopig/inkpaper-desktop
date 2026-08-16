mod app;
mod protocol;
mod server;
mod transport;

/// `inkpaper-desktop --status <serial-port>`: headless USB status check,
/// useful for verifying a connection without going through the GUI (e.g.
/// scripting, or a machine with no display). Everything else launches the
/// normal window.
fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--status" {
        cli_status(&args[2]);
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Inkpaper Desktop",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::default()))),
    )
}

fn cli_status(port: &str) {
    use transport::usb::{UsbEvent, UsbLink};

    let link = match UsbLink::connect(port) {
        Ok(link) => link,
        Err(err) => {
            eprintln!("failed to open {port}: {err}");
            std::process::exit(1);
        }
    };
    if let Err(err) = link.send(protocol::Command::GetStatus) {
        eprintln!("send failed: {err}");
        std::process::exit(1);
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        match link.event_rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(UsbEvent::Reply(reply)) => {
                println!("{reply:?}");
                return;
            }
            Ok(UsbEvent::Log(line)) => println!("(log) {line}"),
            Ok(UsbEvent::Disconnected(reason)) => {
                eprintln!("disconnected: {reason}");
                std::process::exit(1);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("worker thread gone");
                std::process::exit(1);
            }
        }
    }
    eprintln!("timed out waiting for a reply");
    std::process::exit(1);
}
