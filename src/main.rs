use nih_plug::prelude::*;
use synth::Synth;

fn main() {
    let mut args: Vec<String> = std::env::args().collect();

    // nih_plug connects no MIDI input unless told to, and a double-clicked app gets no arguments.
    if !args.iter().any(|arg| arg.starts_with("--midi-input")) {
        if let Some(port_name) = first_midi_input() {
            eprintln!("Using MIDI input '{port_name}'");
            args.push("--midi-input".into());
            args.push(port_name);
        } else {
            eprintln!("No MIDI input found. Plug in the keyboard, then start the app again.");
        }
    }

    nih_export_standalone_with_args::<Synth, _>(args);
}

fn first_midi_input() -> Option<String> {
    let midi = midir::MidiInput::new("synth port scan").ok()?;
    let port = midi.ports().into_iter().next()?;
    midi.port_name(&port).ok()
}
