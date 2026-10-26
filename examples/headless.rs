use std::{io::Write, time::Duration};
use vrx::{
    Result,
    engine::{Playback, Timing},
    text::{Document, orp::Strategy},
};

fn main() -> Result<()> {
    let document = Document::parse("One word at a time.".into(), &Strategy::Classic)?.shared();
    let mut reader = Playback::new(
        document,
        Timing {
            adaptive: false,
            ..Timing::default()
        },
    );
    reader.play(Duration::ZERO);
    // A native caller waits until an Instant deadline; this example injects time.
    while let Some(ticket) = reader.ticket() {
        writeln!(
            std::io::stdout().lock(),
            "{}: {}",
            reader.index(),
            reader.document().word(reader.index()).unwrap_or("")
        )
        .map_err(|e| e.to_string())?;
        reader.advance(ticket, ticket.deadline);
    }
    Ok(())
}
