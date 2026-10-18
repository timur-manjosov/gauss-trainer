//! The contents of the help overlay as plain data.
//!
//! Kept free of rendering types so that the application can bound scrolling
//! and tests can verify that every documented command really parses.

/// One block of the help screen.
#[derive(Debug, Clone, Copy)]
pub struct Section {
    /// Heading.
    pub title: &'static str,
    /// `(keys or command, explanation)` pairs.
    pub rows: &'static [(&'static str, &'static str)],
}

/// The help screen, top to bottom.
pub const SECTIONS: &[Section] = &[
    Section {
        title: "Zeilenoperationen  (Zeilen zählen ab 1)",
        rows: &[
            (
                "m <i> <c>",
                "Zeile i mit dem Faktor c multiplizieren   (mul, mal)",
            ),
            (
                "a <c> <i> <j>",
                "das c-fache von Zeile i zu Zeile j addieren   (add)",
            ),
            (
                "s <i> <j>",
                "Zeile i mit Zeile j vertauschen   (swap, tausch)",
            ),
            (
                "Zahlen",
                "3 · -3/4 · 0.25 · 0,25 – alles wird exakt umgerechnet",
            ),
        ],
    },
    Section {
        title: "Verlauf",
        rows: &[
            (
                "u  ·  Ctrl+Z",
                "letzten Schritt rückgängig machen   (undo, z)",
            ),
            (
                "r  ·  Ctrl+Y",
                "rückgängig gemachten Schritt wiederholen   (redo, y)",
            ),
            (
                "reset  ·  Ctrl+R",
                "zurück zu A₀ – mit Wiederholen wieder vor",
            ),
            ("↑ ↓", "frühere Eingaben abrufen"),
        ],
    },
    Section {
        title: "Aufgaben",
        rows: &[
            (
                "n [MxN] [Art] [seed=K]",
                "neue Matrix   (new, neu · Ctrl+N)",
            ),
            (
                "Art",
                "allgemein · a11 (a₁₁ = 0) · abhängig (Nullzeile) · breit · hoch",
            ),
            ("set [Matrix]", "eigene Matrix eingeben   (e, eingabe)"),
            ("", "Format: 1 2 3; 4 5 6   oder   [[1,2,3],[4,5,6]]"),
        ],
    },
    Section {
        title: "Lernhilfen  (nur auf Anfrage)",
        rows: &[
            (
                "t  ·  F2  ·  Ctrl+G",
                "Tipp, jedes Mal eine Stufe genauer (1 Frage · 2 Art · 3 Schritt)",
            ),
            ("lösung", "Musterlösung – erst nach Bestätigung"),
            (
                "c [Matrix]  ·  F3",
                "eigene Rechnung prüfen (gegen A_k und Treppennormalform)",
            ),
            (
                "analyse  ·  Ctrl+L",
                "Live-Analyse an/aus (Pivots, Pivotspalten, Rang)",
            ),
            ("exam  ·  F4", "Prüfungsmodus: keine Hilfen, Timer läuft"),
        ],
    },
    Section {
        title: "Ansicht",
        rows: &[
            ("theme  ·  Ctrl+T", "dunkles / helles Farbschema"),
            (
                "v  ·  F5",
                "Brüche gestapelt / in einer Zeile / automatisch",
            ),
            (
                "PgUp PgDn  ·  Ctrl+←→↑↓",
                "Matrix scrollen, wenn sie nicht ins Fenster passt",
            ),
            ("?  ·  F1  ·  h", "diese Hilfe     Esc schließt"),
            ("q  ·  Ctrl+Q  ·  Ctrl+C", "Programm beenden"),
        ],
    },
    Section {
        title: "Farben der Live-Analyse",
        rows: &[
            ("grün", "Pivot ist bereits 1"),
            ("gelb", "Pivot, der noch zu 1 werden muss"),
            (
                "rot",
                "Eintrag in einer Pivotspalte, der noch 0 werden muss",
            ),
            ("Spaltenband", "Pivotspalte"),
            ("Zeilenband", "durch den letzten Schritt geänderte Zeile"),
        ],
    },
];

#[cfg(test)]
mod tests {
    use crate::parse::parse_command;

    #[test]
    fn every_documented_command_parses() {
        for line in [
            "m 1 -1/2",
            "mul 1 2",
            "mal 1 2",
            "a -2 1 2",
            "add 1 2 3",
            "s 1 2",
            "swap 1 2",
            "tausch 1 2",
            "u",
            "undo",
            "z",
            "r",
            "redo",
            "y",
            "reset",
            "n",
            "new",
            "neu",
            "n 3x4 a11 seed=1",
            "e",
            "eingabe",
            "set",
            "t",
            "tipp",
            "lösung",
            "c",
            "check",
            "analyse",
            "exam",
            "theme",
            "v",
            "?",
            "h",
            "q",
        ] {
            assert!(parse_command(line).is_ok(), "{line}");
        }
    }
}
