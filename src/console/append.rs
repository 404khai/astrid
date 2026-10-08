use super::state::Piece;
use std::io::{self, Write};

pub(super) fn write_piece(
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    piece: &Piece,
    color: bool,
) -> io::Result<()> {
    let bytes = piece.ink.paint(&piece.text, color);
    if piece.model {
        stdout.write_all(bytes.as_bytes())?;
        stdout.flush()
    } else {
        stderr.write_all(bytes.as_bytes())?;
        stderr.flush()
    }
}
