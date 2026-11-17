//! COBOL(固定/自由形式)の小さな部分集合 → 共通IR → 各スタックのコード。
//! 対応: PROGRAM-ID / 01・77(PIC, VALUE) / DISPLAY / MOVE / ADD / STOP RUN。
//! 未対応の文は TODO コメントとして出力し、黙って落とさない。
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Display(Vec<String>),
    Move(String, String),
    Add(String, String),
    Stop,
    Todo(String),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Var {
    pub name: String,
    pub numeric: bool,
    pub init: String,
}
#[derive(Debug, Default, PartialEq)]
pub struct Program {
    pub id: String,
    pub vars: Vec<Var>,
    pub stmts: Vec<Stmt>,
}

fn strip(src: &str, free: bool) -> Vec<String> {
    src.lines()
        .filter_map(|l| {
            let l: String = if free {
                l.to_string()
            } else {
                let c: Vec<char> = l.chars().collect();
                if c.len() <= 6 || c[6] == '*' || c[6] == '/' {
                    return None;
                }
                c[6..c.len().min(72)].iter().collect()
            };
            if l.trim_start().starts_with("*>") || l.trim().is_empty() {
                None
            } else {
                Some(l)
            }
        })
        .collect()
}

fn ident(n: &str) -> String {
    n.to_lowercase().replace('-', "_")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hello_roundtrip() {
        let src = include_str!("../tests/samples/hello.cbl");
        let p = parse(src, false);
        assert_eq!(p.id, "HELLO");
        assert_eq!(p.vars.len(), 2);
        let r = to_rust(&p);
        assert!(r.contains("format!(\"{}{}\\n\", \"HELLO \", ws_name)"), "{r}");
        assert!(r.contains("ws_count += 2;"));
        assert!(to_python(&p).contains("ws_count += 2"));
    }
}

fn operand(t: &str) -> String {
    let c = t.chars().next().unwrap_or(' ');
    if c == '"' || c == '\'' || c.is_ascii_digit() {
        t.to_string()
    } else {
        ident(t)
    }
}
fn end(t: &str) -> bool {
    t.ends_with('.')
}

/// 空白区切り。引用符内の空白は保持し、末尾の '.' は直後のトークン末尾として残す。
fn tokenize(s: &str) -> Vec<String> {
    let mut v = vec![];
    let mut cur = String::new();
    let mut q: Option<char> = None;
    for c in s.chars() {
        match q {
            Some(qc) => {
                cur.push(c);
                if c == qc {
                    q = None;
                }
            }
            None if c == '"' || c == '\'' => {
                cur.push(c);
                q = Some(c);
            }
            None if c.is_whitespace() => {
                if !cur.is_empty() {
                    v.push(std::mem::take(&mut cur));
                }
            }
            None => cur.push(c),
        }
    }
    if !cur.is_empty() {
        v.push(cur);
    }
    v
}

pub fn parse(src: &str, free: bool) -> Program {
    let text = strip(src, free).join(" ");
    let toks = tokenize(&text);
    let mut p = Program::default();
    let mut i = 0;
    let mut in_proc = false;
    while i < toks.len() {
        let t = toks[i].trim_end_matches('.').to_uppercase();
        match t.as_str() {
            "PROGRAM-ID" => {
                p.id = toks.get(i + 1).map(|s| s.trim_end_matches('.').to_string()).unwrap_or_default();
                i += 2;
            }
            "PROCEDURE" => {
                in_proc = true;
                i += 2;
            }
            "01" | "77" if !in_proc => {
                let name = toks.get(i + 1).cloned().unwrap_or_default().trim_end_matches('.').to_string();
                let mut j = i + 2;
                while j < toks.len() && !end(&toks[j]) {
                    j += 1;
                }
                let seg = toks[(i + 2).min(toks.len())..(j + 1).min(toks.len())].join(" ");
                let mut numeric = false;
                let mut init = String::new();
                let mut it = seg.trim_end_matches('.').split_whitespace();
                while let Some(w) = it.next() {
                    match w.to_uppercase().as_str() {
                        "PIC" | "PICTURE" => {
                            if let Some(pc) = it.next() {
                                let u = pc.to_uppercase();
                                numeric = u.starts_with('9') || u.starts_with("S9");
                            }
                        }
                        "VALUE" => {
                            if let Some(v) = it.next() {
                                init = v.to_string();
                            }
                        }
                        _ => {}
                    }
                }
                p.vars.push(Var { name, numeric, init });
                i = j + 1;
            }
            "DISPLAY" if in_proc => {
                let mut j = i + 1;
                let mut parts = vec![];
                while j < toks.len() {
                    let e = end(&toks[j]);
                    parts.push(operand(toks[j].trim_end_matches('.')));
                    j += 1;
                    if e {
                        break;
                    }
                }
                p.stmts.push(Stmt::Display(parts));
                i = j;
            }
            "MOVE" | "ADD" if in_proc => {
                let ok = toks.get(i + 2).map_or(false, |x| x.eq_ignore_ascii_case("TO")) && toks.get(i + 3).is_some();
                if ok {
                    let a = operand(toks[i + 1].as_str());
                    let b = ident(toks[i + 3].trim_end_matches('.'));
                    p.stmts.push(if t == "MOVE" { Stmt::Move(a, b) } else { Stmt::Add(a, b) });
                    i += 4;
                } else {
                    p.stmts.push(Stmt::Todo(toks[i].clone()));
                    i += 1;
                }
            }
            "STOP" if in_proc => {
                p.stmts.push(Stmt::Stop);
                i += 2;
            }
            _ => {
                let w = toks[i].trim_end_matches('.');
                let verb = in_proc
                    && w.len() > 3
                    && w.chars().all(|c| c.is_ascii_uppercase() || c == '-')
                    && matches!(w, "IF" | "PERFORM" | "COMPUTE" | "SUBTRACT" | "MULTIPLY" | "DIVIDE" | "READ" | "WRITE" | "OPEN" | "CLOSE" | "CALL" | "EVALUATE" | "ACCEPT" | "GOBACK" | "EXEC");
                if verb {
                    p.stmts.push(Stmt::Todo(w.to_string()));
                }
                i += 1;
            }
        }
    }
    p
}

pub fn to_rust(p: &Program) -> String {
    let mut o = String::from(
        "// COBOL_to generated. 数値は i64、文字は String。\n#[allow(unused_assignments, unused_mut, unused_variables)]\npub fn run() -> String {\n    let mut out = String::new();\n",
    );
    for v in &p.vars {
        let n = ident(&v.name);
        if v.numeric {
            o += &format!("    let mut {n}: i64 = {};\n", if v.init.is_empty() { "0" } else { &v.init });
        } else {
            o += &format!("    let mut {n}: String = {}.to_string();\n", if v.init.is_empty() { "\"\"" } else { &v.init });
        }
    }
    for s in &p.stmts {
        match s {
            Stmt::Display(a) => o += &format!("    out += &format!(\"{}\\n\", {});\n", "{}".repeat(a.len()), a.join(", ")),
            Stmt::Move(a, b) => {
                let num = p.vars.iter().any(|v| ident(&v.name) == *b && v.numeric);
                o += &if num { format!("    {b} = {a};\n") } else { format!("    {b} = {a}.to_string();\n") };
            }
            Stmt::Add(a, b) => o += &format!("    {b} += {a};\n"),
            Stmt::Stop => o += "    return out;\n",
            Stmt::Todo(t) => o += &format!("    // TODO(COBOL_to): 未対応の文 {t}\n"),
        }
    }
    if !matches!(p.stmts.last(), Some(Stmt::Stop)) {
        o += "    out\n";
    }
    o + "}\n"
}

pub fn to_python(p: &Program) -> String {
    let mut o = String::from("# COBOL_to generated\ndef run() -> str:\n    out = []\n");
    for v in &p.vars {
        let init = if v.init.is_empty() { if v.numeric { "0".to_string() } else { "\"\"".to_string() } } else { v.init.clone() };
        o += &format!("    {} = {init}\n", ident(&v.name));
    }
    for s in &p.stmts {
        match s {
            Stmt::Display(a) => o += &format!("    out.append(\"\".join(str(x) for x in [{}]))\n", a.join(", ")),
            Stmt::Move(a, b) => o += &format!("    {b} = {a}\n"),
            Stmt::Add(a, b) => o += &format!("    {b} += {a}\n"),
            Stmt::Stop => o += "    return \"\\n\".join(out) + \"\\n\"\n",
            Stmt::Todo(t) => o += &format!("    # TODO(COBOL_to): 未対応の文 {t}\n"),
        }
    }
    o + "    return \"\\n\".join(out) + \"\\n\"\n"
}
