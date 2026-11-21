use origami_syntax::{Code, LineEnding, LineIndex, SourceText};

#[test]
fn lf_roundtrip() {
    let s = SourceText::decode(b"a\nb\n").unwrap();
    assert_eq!(s.text(), "a\nb\n");
    assert_eq!(s.line_ending(), LineEnding::Lf);
    assert_eq!(s.encode(s.text()), b"a\nb\n");
}

#[test]
fn crlf_is_normalized_and_restored() {
    let s = SourceText::decode(b"a\r\nb\r\n").unwrap();
    assert_eq!(s.text(), "a\nb\n");
    assert_eq!(s.line_ending(), LineEnding::CrLf);
    assert_eq!(s.encode(s.text()), b"a\r\nb\r\n");
}

#[test]
fn mixed_endings_follow_first_break() {
    let s = SourceText::decode(b"a\r\nb\nc\r\n").unwrap();
    assert_eq!(s.text(), "a\nb\nc\n");
    assert_eq!(s.encode(s.text()), b"a\r\nb\r\nc\r\n");
}

#[test]
fn lone_cr_is_kept() {
    let s = SourceText::decode(b"a\rb\n").unwrap();
    assert_eq!(s.text(), "a\rb\n");
    assert_eq!(s.encode(s.text()), b"a\rb\n");
}

#[test]
fn bom_is_stripped_and_restored() {
    let s = SourceText::decode(b"\xEF\xBB\xBFabc\n").unwrap();
    assert!(s.has_bom());
    assert_eq!(s.text(), "abc\n");
    assert_eq!(s.encode(s.text()), b"\xEF\xBB\xBFabc\n");
}

#[test]
fn final_newline_state_is_restored() {
    let without = SourceText::decode(b"abc").unwrap();
    assert!(!without.has_final_newline());
    assert_eq!(without.encode("abc\n"), b"abc");
    let with = SourceText::decode(b"abc\n").unwrap();
    assert_eq!(with.encode("abc"), b"abc\n");
}

#[test]
fn invalid_utf8_is_e005() {
    let err = SourceText::decode(b"ab\xFFcd").unwrap_err();
    assert_eq!(err.code, Code::E005);
    assert_eq!(err.start, 2);
}

#[test]
fn utf16_columns() {
    let text = "ä😀x\nzz";
    let idx = LineIndex::new(text);
    let x = text.find('x').unwrap() as u32;
    let p = idx.position(x);
    assert_eq!((p.line, p.column), (1, 4)); // ä = 1 unit, 😀 = 2 units
    let z = text.rfind('z').unwrap() as u32;
    assert_eq!((idx.position(z).line, idx.position(z).column), (2, 2));
}
